use std::{error::Error, sync::mpsc};

use airplay::playback::video::{PacketKind, VideoPacket, VideoParams};
use gstreamer::{
    Buffer, Caps, Element, ElementFactory, Format, MessageType, MessageView, Pipeline, State,
    event::Eos, glib::GString, prelude::*,
};
use gstreamer_app::AppSrc;

pub fn transcode(
    id: u64,
    _params: VideoParams,
    rx: mpsc::Receiver<VideoPacket>,
) -> Result<(), Box<dyn Error>> {
    let mut ctx = None;
    loop {
        if let Ok(VideoPacket { kind, payload, .. }) = rx.recv() {
            match kind {
                PacketKind::AvcC => match create_stream(payload, id) {
                    Ok(res) => {
                        ctx = Some(res);
                    }
                    Err(err) => {
                        tracing::error!(%err, "couldn't initialize context with avcc header");
                    }
                },
                PacketKind::Payload => {
                    let Some(ctx) = &ctx else {
                        tracing::warn!("uninitialized context before payload");
                        continue;
                    };

                    let _ = ctx
                        .appsrc
                        .push_buffer(Buffer::from_slice(payload))
                        .inspect_err(|err| tracing::warn!(%err, "packet push failed"));
                }
                PacketKind::Other(kind) => {
                    tracing::debug!(%kind, "unknown packet type");
                }
            }
        } else {
            let Some(ctx) = &ctx else {
                return Ok(());
            };

            ctx.pipeline.send_event(Eos::new());
        }

        let Some(state) = &ctx else {
            continue;
        };

        let bus = state
            .pipeline
            .bus()
            .ok_or("pipeline must have message bus")?;

        for msg in bus.iter_filtered(&[MessageType::Eos, MessageType::Error]) {
            match msg.view() {
                MessageView::Eos(..) => {
                    return Ok(());
                }
                MessageView::Error(err) => {
                    return Err(format!(
                        "Error from {:?}: {} (debug: {:?})",
                        msg.src()
                            .map_or_else(|| GString::from("UNKNOWN"), GstObjectExt::path_string),
                        err.error(),
                        err.debug(),
                    ).into());
                }
                _ => {}
            }
        }
    }
}

fn build_passthrough_pipeline(
    pipeline: &Pipeline,
    appsrc: &Element,
    parser: &str,
    file_location: &str,
) -> Result<(), Box<dyn Error>> {
    let ingest_parse = ElementFactory::make(parser).build()?;
    let muxer = ElementFactory::make("mp4mux").build()?;
    let sink = ElementFactory::make("filesink")
        .property("location", file_location)
        .build()?;

    pipeline.add_many([appsrc, &ingest_parse, &muxer, &sink])?;
    Element::link_many([appsrc, &ingest_parse, &muxer, &sink])?;

    Ok(())
}

fn detect_codec(avcc: &[u8]) -> VideoCodec {
    let avcc_ref = avcc.as_ref();

    if avcc_ref.len() >= 8 {
        return match &avcc_ref[4..8] {
            b"hvc1" => {
                VideoCodec::H265
            }
            _ => {
                VideoCodec::H264
            }
        }
    }

    VideoCodec::Unknown
}

fn extract_codec_record(header: &[u8], codec: VideoCodec) -> Option<Vec<u8>> {
    if header.first().copied() == Some(1) {
        return Some(header.to_vec());
    }

    let marker = match codec {
        VideoCodec::H264 => b"avcC",
        VideoCodec::H265 => b"hvcC",
        VideoCodec::Unknown => return None,
    };

    find_box_payload(header, marker)
}

fn find_box_payload(buf: &[u8], marker: &[u8; 4]) -> Option<Vec<u8>> {
    const VIDEO_SAMPLE_ENTRY_FIELDS_LEN: usize = 78;

    fn parse_range(
        buf: &[u8],
        mut cursor: usize,
        limit: usize,
        marker: &[u8; 4],
    ) -> Option<Vec<u8>> {
        while cursor + 8 <= limit {
            let mut size = u32::from_be_bytes(buf[cursor..cursor + 4].try_into().ok()?) as usize;
            if size == 0 {
                size = limit - cursor;
            }
            if size < 8 || cursor + size > limit {
                return None;
            }

            let kind = &buf[cursor + 4..cursor + 8];
            if kind == marker {
                return Some(buf[cursor + 8..cursor + size].to_vec());
            }

            if matches!(kind, b"avc1" | b"hvc1" | b"hev1") {
                let body_start = cursor + 8 + VIDEO_SAMPLE_ENTRY_FIELDS_LEN;
                if body_start < cursor + size {
                    if let Some(inner) = parse_range(buf, body_start, cursor + size, marker) {
                        return Some(inner);
                    }
                }
            } else if matches!(kind, b"stsd" | b"trak" | b"mdia" | b"minf" | b"stbl") {
                // generic container boxes that may wrap sample entries
                let body_start = cursor + 8;
                if body_start < cursor + size {
                    if let Some(inner) = parse_range(buf, body_start, cursor + size, marker) {
                        return Some(inner);
                    }
                }
            }

            cursor += size;
        }
        None
    }

    parse_range(buf, 0, buf.len(), marker)
}

fn format_header_snapshot(avcc: &[u8]) -> String {
    avcc.iter()
        .take(16)
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn create_stream(
    avcc: impl AsRef<[u8]> + Send + 'static,
    id: u64,
) -> Result<Context, Box<dyn Error>> {
    let header = avcc.as_ref().to_vec();
    let codec = detect_codec(&header);
    match codec {
        VideoCodec::H264 => println!("stream {id}: detected H.264"),
        VideoCodec::H265 => println!("stream {id}: detected H.265"),
        VideoCodec::Unknown => println!(
            "stream {id}: codec unknown (len={}, header={})",
            header.len(),
            format_header_snapshot(&header)
        ),
    }

    let spec = CodecPipelineSpec::from(codec);
    let pipeline = Pipeline::default();
    let codec_data = extract_codec_record(&header, codec).unwrap_or_else(|| header.clone());

    let caps = Caps::builder(spec.caps_mime)
        .field("stream-format", spec.stream_format)
        .field("alignment", "au")
        .field("codec_data", Buffer::from_slice(codec_data))
        .build();

    let appsrc = AppSrc::builder()
        .caps(&caps)
        .format(Format::Time)
        .is_live(true)
        .do_timestamp(true)
        .build();

    let file_location = format!("video_{id}.mp4");
    build_passthrough_pipeline(&pipeline, appsrc.upcast_ref(), spec.parser, &file_location)?;

    pipeline.set_state(State::Playing)?;

    Ok(Context { pipeline, appsrc })
}

struct Context {
    pipeline: Pipeline,
    appsrc: AppSrc,
}

impl Drop for Context {
    fn drop(&mut self) {
        if let Err(err) = self.pipeline.set_state(State::Null) {
            tracing::warn!(%err, "pipeline state failed to be set to null");
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum VideoCodec {
    H264,
    H265,
    Unknown,
}

struct CodecPipelineSpec {
    caps_mime: &'static str,
    stream_format: &'static str,
    parser: &'static str,
}

impl CodecPipelineSpec {
    fn from(codec: VideoCodec) -> Self {
        match codec {
            VideoCodec::H265 => Self {
                caps_mime: "video/x-h265",
                stream_format: "hvc1",
                parser: "h265parse",
            },
            _ => Self {
                caps_mime: "video/x-h264",
                stream_format: "avc",
                parser: "h264parse",
            },
        }
    }
}

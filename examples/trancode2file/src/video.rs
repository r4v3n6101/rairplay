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
                PacketKind::AvcC | PacketKind::HvcC => match create_stream(payload, id) {
                    Ok(res) => {
                        ctx = Some(res);
                    }
                    Err(err) => {
                        tracing::error!(%err, "couldn't initialize context with codec config header");
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
                },
                PacketKind::Plist => {
                    tracing::debug!("plist packet received");
                },
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
                    )
                    .into());
                }
                _ => {}
            }
        }
    }
}

fn create_stream(
    avcc: impl AsRef<[u8]> + Send + 'static,
    id: u64,
) -> Result<Context, Box<dyn Error>> {
    let pipeline = Pipeline::default();

    let caps = Caps::builder("video/x-h264")
        .field("stream-format", "avc")
        .field("alignment", "au")
        .field("codec_data", Buffer::from_slice(avcc))
        .build();

    let appsrc = AppSrc::builder()
        .caps(&caps)
        .format(Format::Time)
        .is_live(true)
        .do_timestamp(true)
        .build();

    let h264parse_in = ElementFactory::make("h264parse").build()?;
    let decoder = ElementFactory::make("avdec_h264").build()?;
    let videoconvert = ElementFactory::make("videoconvert").build()?;
    let encoder = ElementFactory::make("x264enc").build()?;
    let h264parse_out = ElementFactory::make("h264parse").build()?;
    let muxer = ElementFactory::make("mp4mux").build()?;
    let sink = ElementFactory::make("filesink")
        .property("location", format!("video_{id}.mp4"))
        .build()?;

    pipeline.add_many([
        appsrc.upcast_ref(),
        &h264parse_in,
        &decoder,
        &videoconvert,
        &encoder,
        &h264parse_out,
        &muxer,
        &sink,
    ])?;
    Element::link_many([
        appsrc.upcast_ref(),
        &h264parse_in,
        &decoder,
        &videoconvert,
        &encoder,
        &h264parse_out,
        &muxer,
        &sink,
    ])?;

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

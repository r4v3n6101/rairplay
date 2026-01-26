use std::{
    error::Error,
    sync::{mpsc, Arc, Mutex, OnceLock},
    thread,
};

use airplay::playback::video::{PacketKind, VideoPacket, VideoParams};
use gpui::RenderImage;
use gstreamer::{
    event::Eos,
    glib::{object::Cast, GString},
    prelude::{ElementExt, ElementExtManual, GstBinExtManual, GstObjectExt},
    Buffer, Caps, Element, ElementFactory, Format, MessageType, MessageView, Pipeline, State,
};
use gstreamer_app::AppSrc;
use gstreamer_video::{prelude::VideoFrameExt, VideoFormat, VideoFrame, VideoInfo};
use image::{Frame, RgbaImage};
use smallvec::SmallVec;
use tokio::sync::mpsc::UnboundedSender;

use crate::ui::UiEvent;

pub struct FrameState {
    pub image: Option<Arc<RenderImage>>,
    pub size: Option<(u32, u32)>,
}

pub type SharedFrame = Arc<Mutex<FrameState>>;

static SHARED_FRAME: OnceLock<SharedFrame> = OnceLock::new();
static UI_EVENTS: OnceLock<UnboundedSender<UiEvent>> = OnceLock::new();

pub fn init_shared_frame() -> SharedFrame {
    let shared = Arc::new(Mutex::new(FrameState {
        image: None,
        size: None,
    }));
    let _ = SHARED_FRAME.set(Arc::clone(&shared));
    shared
}

pub fn set_ui_sender(sender: UnboundedSender<UiEvent>) {
    let _ = UI_EVENTS.set(sender);
}

fn shared_frame() -> SharedFrame {
    SHARED_FRAME
        .get()
        .expect("shared frame must be initialized")
        .clone()
}

fn send_ui(event: UiEvent) {
    if let Some(sender) = UI_EVENTS.get() {
        let _ = sender.send(event);
    }
}

pub fn transcode(
    _id: u64,
    _params: VideoParams,
    rx: mpsc::Receiver<VideoPacket>,
) -> Result<(), Box<dyn Error>> {
    let shared_frame = shared_frame();
    let mut ui_session = UiSession::new();

    let mut ctx = None;
    loop {
        if let Ok(VideoPacket { kind, payload, .. }) = rx.recv() {
            match kind {
                PacketKind::AvcC => match create_stream(payload, shared_frame.clone()) {
                    Ok(res) => {
                        tracing::debug!("avcc packet type");
                        ui_session.open();
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
    shared_frame: SharedFrame,
) -> Result<ContextState, Box<dyn Error>> {
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

    let h264parse = ElementFactory::make("h264parse").build()?;
    let decoder = ElementFactory::make("avdec_h264").build()?;
    let convert = ElementFactory::make("videoconvert").build()?;
    let appsink = ElementFactory::make("appsink")
        .property("emit-signals", false)
        .property("sync", false)
        .build()?;

    pipeline.add_many([
        appsrc.upcast_ref(),
        &h264parse,
        &decoder,
        &convert,
        &appsink,
    ])?;
    Element::link_many([
        appsrc.upcast_ref(),
        &h264parse,
        &decoder,
        &convert,
        &appsink,
    ])?;

    let appsink = appsink
        .dynamic_cast::<gstreamer_app::AppSink>()
        .map_err(|_| "failed to cast appsink")?;

    spawn_decoder_thread(appsink, shared_frame);

    pipeline.set_state(State::Playing)?;

    Ok(ContextState { pipeline, appsrc })
}

struct ContextState {
    pipeline: Pipeline,
    appsrc: AppSrc,
}

impl Drop for ContextState {
    fn drop(&mut self) {
        if let Err(err) = self.pipeline.set_state(State::Null) {
            tracing::warn!(%err, "pipeline state failed to be set to null");
        }
    }
}

struct UiSession {
    opened: bool,
}

impl UiSession {
    fn new() -> Self {
        Self { opened: false }
    }

    fn open(&mut self) {
        if !self.opened {
            send_ui(UiEvent::Open);
            self.opened = true;
        }
    }
}

impl Drop for UiSession {
    fn drop(&mut self) {
        if self.opened {
            send_ui(UiEvent::Close);
        }
    }
}

fn spawn_decoder_thread(appsink: gstreamer_app::AppSink, shared_frame: SharedFrame) {
    thread::spawn(move || {
        let format_caps = Caps::builder("video/x-raw").field("format", "BGRA").build();
        appsink.set_caps(Some(&format_caps));

        loop {
            let sample = match appsink.pull_sample() {
                Ok(sample) => sample,
                Err(_) => break,
            };
            let buffer = match sample.buffer() {
                Some(buffer) => buffer,
                None => continue,
            };
            let caps = match sample.caps() {
                Some(caps) => caps,
                None => continue,
            };

            let info = match VideoInfo::from_caps(&caps) {
                Ok(info) => info,
                Err(_) => continue,
            };

            if info.format() != VideoFormat::Bgra {
                continue;
            }

            let buffer = buffer.copy();
            let frame = match VideoFrame::from_buffer_readable(buffer, &info) {
                Ok(frame) => frame,
                Err(_) => continue,
            };
            let data = match frame.plane_data(0) {
                Ok(data) => data,
                Err(_) => continue,
            };

            let width = info.width();
            let height = info.height();
            let stride = frame.plane_stride()[0] as usize;
            let row_len = (width as usize) * 4;
            let mut rgba = vec![0u8; row_len * height as usize];

            for y in 0..height as usize {
                let src = &data[y * stride..y * stride + row_len];
                let dst = &mut rgba[y * row_len..(y + 1) * row_len];
                dst.copy_from_slice(src);
            }

            let image = match RgbaImage::from_vec(width, height, rgba) {
                Some(image) => image,
                None => continue,
            };

            let frame = Frame::new(image);
            let render = Arc::new(RenderImage::new(SmallVec::from_elem(frame, 1)));
            let mut state = shared_frame.lock().unwrap();
            state.image = Some(render);
            state.size = Some((width, height));
        }
    });
}

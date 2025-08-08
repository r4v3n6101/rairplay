use std::{error::Error, sync::mpsc};

use gstreamer::{
    event::Eos,
    glib::{object::Cast, GString},
    prelude::{ElementExt, ElementExtManual, GstBinExtManual, GstObjectExt},
    Buffer, Caps, Element, ElementFactory, Format, MessageType, MessageView, Pipeline, State,
};
use gstreamer_app::AppSrc;
use rairplay::playback::audio::{AudioPacket, AudioParams};

pub fn transcode(
    id: u64,
    params: AudioParams,
    rx: mpsc::Receiver<AudioPacket>,
) -> Result<(), Box<dyn Error>> {
    let ctx = create_stream(&params, id)?;
    loop {
        if let Ok(packet) = rx.recv() {
            let mut rtp = packet.rtp;

            // TODO : only for AAC?
            rtp[1] |= 0b1000_0000;

            let _ = ctx
                .appsrc
                .push_buffer(Buffer::from_slice(rtp))
                .inspect_err(|err| tracing::warn!(%err, "packet push failed"));
        } else {
            ctx.pipeline.send_event(Eos::new());
        }

        let bus = ctx.pipeline.bus().ok_or("pipeline must have message bus")?;

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

fn create_stream(params: &AudioParams, id: u64) -> Result<Context, Box<dyn Error>> {
    let pipeline = Pipeline::default();

    let caps = Caps::builder("application/x-rtp")
        .field("media", "audio")
        .field("mode", "generic")
        .field("clock-rate", 44100i32)
        .field("encoding-name", "MPEG4-GENERIC")
        .field("channels", 2i32)
        .field("config", "1210")
        .field("constantduration", 1024)
        .build();

    let appsrc = AppSrc::builder()
        .caps(&caps)
        .format(Format::Time)
        .is_live(true)
        .do_timestamp(true)
        .build();

    let jitterbuffer = ElementFactory::make("rtpjitterbuffer").build()?;
    let mp4gdepay_rs = ElementFactory::make("rtpmp4gdepay2").build()?;
    let aacparse = ElementFactory::make("aacparse").build()?;
    let avdec_aac = ElementFactory::make("avdec_aac").build()?;
    let convert = ElementFactory::make("audioconvert").build()?;
    let resample = ElementFactory::make("audioresample").build()?;
    let wavenc = ElementFactory::make("wavenc").build()?;
    let sink = ElementFactory::make("filesink")
        .property("location", format!("audio_{id}.wav"))
        .build()?;

    pipeline.add_many([
        appsrc.upcast_ref(),
        &jitterbuffer,
        &mp4gdepay_rs,
        &aacparse,
        &avdec_aac,
        &convert,
        &resample,
        &wavenc,
        &sink,
    ])?;
    Element::link_many([
        appsrc.upcast_ref(),
        &jitterbuffer,
        &mp4gdepay_rs,
        &aacparse,
        &avdec_aac,
        &convert,
        &resample,
        &wavenc,
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

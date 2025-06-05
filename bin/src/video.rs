use std::{
    io::{self, Write},
    sync::mpsc,
};

use ffmpeg_next::{codec, decoder, error::Error as FFError, ffi, frame, Packet};
use rairplay::playback::video::{PacketKind, VideoPacket};

pub fn transcode(rx: mpsc::Receiver<VideoPacket>, mut output: impl Write) -> Result<(), FFError> {
    let mut decoder = codec::Context::new_with_codec(
        decoder::find(codec::Id::H264).expect("H264 must be supported by ffmpeg"),
    )
    .decoder()
    .video()?;
    let mut eof = false;

    loop {
        match rx.recv() {
            Ok(pkt) if matches!(pkt.kind, PacketKind::AvcC) => {
                decoder = create_h264_decoder(&pkt.payload)?;
            }
            Ok(pkt) if matches!(pkt.kind, PacketKind::Payload) => {
                decoder.send_packet(&Packet::copy(&pkt.payload))?;
            }
            Err(_) => {
                decoder.send_eof()?;
                eof = true;
            }
            _ => {}
        }

        let mut frame = frame::Video::empty();
        while decoder.receive_frame(&mut frame).is_ok() {
            match write_yuv420p_frame(&frame, &mut output) {
                Ok(_) => {
                    tracing::trace!(width=%frame.width(), height=%frame.height(), "frame written");
                }
                Err(err) => {
                    tracing::warn!(%err, width=%frame.width(), height=%frame.height(), fmt=?frame.format(), "frame couldn't be written");
                }
            }
        }

        if eof {
            return Ok(());
        }
    }
}

fn create_h264_decoder(avcc: &[u8]) -> Result<decoder::Video, FFError> {
    let mut ctx = codec::Context::new_with_codec(
        decoder::find(codec::Id::H264).expect("H264 must be supported by ffmpeg"),
    );
    let mut params = codec::Parameters::new();

    // Safety: safe, but there's no high level binding for that
    unsafe {
        let params = &mut *params.as_mut_ptr();

        // avcC header must reside in extra data
        params.extradata =
            ffi::av_malloc(avcc.len() + ffi::AV_INPUT_BUFFER_PADDING_SIZE as usize).cast::<u8>();
        params.extradata_size = avcc.len() as _;
        params.extradata.copy_from(avcc.as_ptr(), avcc.len());
    }
    ctx.set_parameters(params)?;

    ctx.decoder().video()
}

fn write_yuv420p_frame(frame: &frame::Video, to: &mut impl Write) -> io::Result<()> {
    let width = frame.width();
    let height = frame.height();

    // Write Y plane
    for y in 0..height {
        let line = &frame.data(0)[(y as usize * frame.stride(0))..][..width as usize];
        to.write_all(line)?;
    }

    // Write U plane
    for y in 0..(height / 2) {
        let line = &frame.data(1)[(y as usize * frame.stride(1))..][..(width / 2) as usize];
        to.write_all(line)?;
    }

    // Write V plane
    for y in 0..(height / 2) {
        let line = &frame.data(2)[(y as usize * frame.stride(2))..][..(width / 2) as usize];
        to.write_all(line)?;
    }

    Ok(())
}

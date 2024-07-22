use std::io;

use bytes::BytesMut;
use symphonia::{
    core::{
        audio::{Layout, RawSampleBuffer, SignalSpec},
        codecs::{CodecParameters, Decoder, DecoderOptions, CODEC_TYPE_AAC},
        formats::Packet,
    },
    default::codecs::AacDecoder,
};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, ToSocketAddrs, UdpSocket},
};

use self::{crypt::ChaCha20Poly1305Cipher, packet::RtpPacket};

use super::Handle;

pub mod crypt;
pub mod packet;

// TODO : this is a big logic of code and must be moved out w/ realtime one
pub async fn spawn_buffered(
    bind_addr: impl ToSocketAddrs,
    shared_key: &[u8],
) -> io::Result<Handle> {
    const AUDIO_BUF_LEN: usize = 4 * 1024 * 1024;

    let listener = TcpListener::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;

    // TODO
    let cipher = ChaCha20Poly1305Cipher::new(shared_key).unwrap();
    let handle = tokio::spawn(async move {
        // TODO : replace all while's with loop and check errors
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut audio_buf = BytesMut::zeroed(AUDIO_BUF_LEN);
            let mut pkts = Vec::new();

            while let Ok(pkt_len) = stream.read_u16().await {
                // 2 is pkt_len size itself
                let pkt_len: usize = pkt_len.wrapping_sub(2).into();
                if pkt_len < RtpPacket::base_len() {
                    tracing::warn!(%pkt_len, "malformed rtp packet");
                    continue;
                }

                let payload_len = pkt_len - RtpPacket::base_len();
                if audio_buf.len() < payload_len {
                    // TODO : test block for checking correctness of work
                    {
                        let mut decoder = AacDecoder::try_new(
                            CodecParameters::new()
                                .for_codec(CODEC_TYPE_AAC)
                                .with_sample_rate(44100)
                                .with_channel_layout(Layout::Stereo),
                            &DecoderOptions::default(),
                        )
                        .unwrap();

                        let mut buf = Vec::new();
                        for pkt in &mut pkts {
                            match cipher.decrypt_pkt(pkt) {
                                Ok(clear_data) => {
                                    let pack = Packet::new_from_slice(0, 0, 0, clear_data);
                                    let audioref = decoder.decode(&pack).unwrap();
                                    let mut sample_buf = RawSampleBuffer::<u16>::new(
                                        1024,
                                        SignalSpec::new_with_layout(44100, Layout::Stereo),
                                    );
                                    sample_buf.copy_interleaved_ref(audioref);
                                    buf.extend_from_slice(sample_buf.as_bytes());
                                }
                                Err(err) => {
                                    tracing::warn!(%err, "couldn't decrypt rtp packet");
                                }
                            }
                        }
                        std::fs::write("./raw_u16le_44100_2.pcm", buf);
                        tracing::error!("done!");
                    }
                    audio_buf = BytesMut::zeroed(AUDIO_BUF_LEN);
                    tracing::warn!("audio buffer reallocated");
                }

                let mut header = [0u8; RtpPacket::header_len()];
                let mut trailer = [0u8; RtpPacket::trailer_len()];
                let mut payload = audio_buf.split_to(payload_len);

                match stream.read_exact(&mut header).await {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::warn!(%err, %pkt_len, "failed to read rtp header");
                        continue;
                    }
                };
                match stream.read_exact(&mut payload).await {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::warn!(%err, %pkt_len, "failed to read rtp payload");
                        continue;
                    }
                };
                match stream.read_exact(&mut trailer).await {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::warn!(%err, %pkt_len, "failed to read rtp trailer");
                        continue;
                    }
                };

                let rtp_pkt = RtpPacket::new(header, trailer, payload);
                pkts.push(rtp_pkt);
            }
        }
    })
    .abort_handle();

    Ok(Handle { handle, local_addr })
}

pub async fn spawn_control(bind_addr: impl ToSocketAddrs) -> io::Result<Handle> {
    let listener = UdpSocket::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;
    let handle = tokio::spawn(async move { while let Ok(_) = listener.recv(&mut []).await {} })
        .abort_handle();

    Ok(Handle { handle, local_addr })
}

pub async fn spawn_realtime(bind_addr: impl ToSocketAddrs) -> io::Result<Handle> {
    let listener = UdpSocket::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;
    let handle = tokio::spawn(async move { while let Ok(_) = listener.recv(&mut []).await {} })
        .abort_handle();

    Ok(Handle { handle, local_addr })
}

use std::io;

use bytes::BytesMut;
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, ToSocketAddrs},
};

use crate::channels::{audio::packet::RtpPacket, Handle};

pub async fn spawn_processor(bind_addr: impl ToSocketAddrs) -> io::Result<Handle> {
    let listener = TcpListener::bind(bind_addr).await?;
    Ok(Handle {
        local_addr: listener.local_addr()?,
        handle: tokio::spawn(task(listener)).abort_handle(),
    })
}

async fn task(listener: TcpListener) {
    const AUDIO_BUF_LEN: usize = 4 * 1024 * 1024;

    // TODO : may be use one listener for all task?
    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let mut audio_buf = BytesMut::zeroed(AUDIO_BUF_LEN);
                while let Ok(pkt_len) = stream.read_u16().await {
                    // 2 is pkt_len size itself
                    let pkt_len: usize = pkt_len.wrapping_sub(2).into();
                    if pkt_len < RtpPacket::base_len() {
                        tracing::warn!(%pkt_len, "malformed rtp packet");
                        continue;
                    }

                    let payload_len = pkt_len - RtpPacket::base_len();
                    if audio_buf.len() < payload_len {
                        audio_buf = BytesMut::zeroed(AUDIO_BUF_LEN);
                        tracing::debug!(size = %audio_buf.len(), "audio buffer reallocated");
                    }

                    let mut header = [0u8; RtpPacket::header_len()];
                    let mut trailer = [0u8; RtpPacket::trailer_len()];
                    let mut payload = audio_buf.split_to(payload_len);

                    if let Err(err) = stream.read_exact(&mut header).await {
                        tracing::warn!(%err, %pkt_len, "failed to read rtp header");
                        continue;
                    };
                    if let Err(err) = stream.read_exact(&mut payload).await {
                        tracing::warn!(%err, %pkt_len, "failed to read rtp payload");
                        continue;
                    };
                    if let Err(err) = stream.read_exact(&mut trailer).await {
                        tracing::warn!(%err, %pkt_len, "failed to read rtp trailer");
                        continue;
                    };

                    let rtp_pkt = RtpPacket::new(header, trailer, payload);
                    // TODO : send it to next processing steps
                }
            }
            Err(err) => {
                tracing::error!(%err, "couldn't accept connection");
            }
        }
    }
}

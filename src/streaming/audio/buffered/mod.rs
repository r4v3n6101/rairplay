use std::io;

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use crate::streaming::{audio::packet::RtpPacket, Handle};

use super::buffer::AudioBuffer;

pub async fn spawn_processor(bind_addr: impl ToSocketAddrs) -> io::Result<Handle> {
    let listener = TcpListener::bind(bind_addr).await?;
    Ok(Handle {
        local_addr: listener.local_addr()?,
        handle: tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, remote_addr)) => {
                        tracing::info!(%remote_addr, "accepted new connection");
                        tokio::spawn(processor(stream));
                    }
                    Err(err) => {
                        tracing::warn!(%err, "failed to accept new connection");
                    }
                }
            }
        })
        .abort_handle(),
    })
}

// TODO : rename
#[tracing::instrument(name = "buffered_audio_processor", level = tracing::Level::DEBUG, skip_all)]
async fn processor(mut stream: TcpStream) {
    let mut audio_buf = AudioBuffer::new();
    while let Ok(pkt_len) = stream.read_u16().await {
        // 2 is pkt_len size itself
        let pkt_len: usize = pkt_len.saturating_sub(2).into();
        if pkt_len < RtpPacket::base_len() {
            tracing::warn!(%pkt_len, "malformed rtp packet");
            continue;
        }

        let payload_len = pkt_len - RtpPacket::base_len();

        let mut header = [0u8; RtpPacket::header_len()];
        let mut trailer = [0u8; RtpPacket::trailer_len()];
        let mut payload = audio_buf.allocate_buf(payload_len);

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

        audio_buf.push_packet(RtpPacket::new(header, trailer, payload));
    }
}

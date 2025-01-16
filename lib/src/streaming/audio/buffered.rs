use std::{io, net::SocketAddr};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use crate::util::BytePool;

use super::{
    super::command,
    packet::{RtpHeader, RtpTrailer},
};

async fn processor(mut stream: TcpStream, audio_buf_size: usize, cmd_handler: command::Handler) {
    let mut audio_buf = BytePool::new(audio_buf_size);

    while let Ok(pkt_len) = stream.read_u16().await {
        // 2 is pkt_len size itself
        let pkt_len: usize = pkt_len.saturating_sub(2).into();

        let Some(payload_len) = pkt_len.checked_sub(RtpHeader::SIZE + RtpTrailer::SIZE) else {
            tracing::warn!(%pkt_len, "malformed rtp packet");
            continue;
        };

        let mut header = RtpHeader::empty();
        let mut trailer = RtpTrailer::empty();
        let mut payload = audio_buf.allocate_buf(payload_len);

        match (
            stream.read_exact(&mut *header).await,
            stream.read_exact(&mut payload).await,
            stream.read_exact(&mut *trailer).await,
        ) {
            (Ok(_), Ok(_), Ok(_)) => {}
            _ => break,
        }

        tracing::debug!(?header, ?trailer, "new rtp packet");
    }
}

pub struct Channel {
    local_addr: SocketAddr,
    audio_buf_size: u32,
}

impl Channel {
    pub async fn create(
        bind_addr: impl ToSocketAddrs,
        audio_buf_size: u32,
        cmd_handler: command::Handler,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        // TODO : here channel with command, it must be shared with realtime audio channel
        let task = async move {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    tracing::info!(%local_addr, %remote_addr, "accepting connection");
                    processor(stream, audio_buf_size as usize, cmd_handler).await;
                    tracing::info!(%local_addr, %remote_addr, "buffered stream done");
                }
                Err(err) => {
                    tracing::warn!(%err, %local_addr, "failed to accept connection");
                }
            }
        };

        tokio::spawn(task);

        Ok(Channel {
            local_addr,
            audio_buf_size,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn audio_buf_size(&self) -> u32 {
        self.audio_buf_size
    }
}

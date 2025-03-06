use std::{io, net::SocketAddr};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use crate::{
    device::{BufferedData, DataCallback},
    util::memory,
};

use super::packet::{RtpHeader, RtpTrailer};

pub struct Channel {
    local_addr: SocketAddr,
    audio_buf_size: u32,
}

impl Channel {
    pub async fn create(bind_addr: impl ToSocketAddrs, audio_buf_size: u32) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        let task = async move {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    tracing::info!(%local_addr, %remote_addr, "accepting connection");
                    processor(stream, audio_buf_size).await;
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

    pub fn data_callback(&self) -> DataCallback<()> {
        // TODO : change to acquiring from jitter buffer or just plain buffer
        Box::new(|| BufferedData {
            wait_until_next: None,
            data: Vec::new(),
        })
    }
}

async fn processor(mut stream: TcpStream, audio_buf_size: u32) {
    let mut audio_buf = memory::BytesHunk::new(audio_buf_size as usize);

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
    }
}

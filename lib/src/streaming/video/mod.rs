use std::{
    io,
    net::SocketAddr,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use tokio::net::{TcpListener, ToSocketAddrs};

use crate::device::{BufferedData, DataChannel, VideoPacket};

mod buffered;
mod packet;

pub struct Channel {
    inner: ChannelInner,
}

#[derive(Clone)]
pub struct ChannelInner {
    pub local_addr: SocketAddr,
    is_alive: Arc<AtomicBool>,
}

impl ChannelInner {
    pub async fn create(
        bind_addr: impl ToSocketAddrs,
        video_buf_size: u32,
        latency: Duration,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        // TODO
        let is_alive = Arc::new(AtomicBool::new(true));
        tokio::spawn(async move {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    tracing::info!(%local_addr, %remote_addr, "accepting connection");
                    buffered::processor(stream, video_buf_size, latency).await;
                    tracing::info!(%local_addr, %remote_addr, "video stream done");
                }
                Err(err) => {
                    tracing::warn!(%err, %local_addr, "failed to accept connection");
                }
            }
        });

        Ok(Self {
            local_addr,
            is_alive,
        })
    }
}

impl DataChannel for Channel {
    type Content = VideoPacket;

    fn pull_data(&self) -> BufferedData<Self::Content> {
        BufferedData {
            wait_until_next: None,
            data: vec![],
        }
    }
}

impl From<ChannelInner> for Channel {
    fn from(value: ChannelInner) -> Self {
        Self { inner: value }
    }
}

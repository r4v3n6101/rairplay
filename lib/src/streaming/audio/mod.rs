use std::{
    io,
    net::SocketAddr,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use tokio::net::{TcpListener, ToSocketAddrs, UdpSocket};

use crate::device::{AudioPacket, BufferedData, DataChannel};

mod buffered;
mod packet;
mod realtime;

#[derive(Clone)]
enum ChannelInner {
    Realtime(RealtimeChannel),
    Buffered(BufferedChannel),
}

#[derive(Clone)]
pub struct Channel {
    inner: ChannelInner,
}

#[derive(Clone)]
pub(crate) struct RealtimeChannel {
    pub local_data_addr: SocketAddr,
    pub local_control_addr: SocketAddr,
    pkt_buf: realtime::SharedPacketBuf,
    // TODO : Notify, because it may be teared down and forget to close chan
    is_alive: Arc<AtomicBool>,
}

#[derive(Clone)]
pub(crate) struct BufferedChannel {
    pub local_addr: SocketAddr,
    pub audio_buf_size: u32,
    is_alive: Arc<AtomicBool>,
}

impl RealtimeChannel {
    pub async fn create(
        data_bind_addr: impl ToSocketAddrs,
        control_bind_addr: impl ToSocketAddrs,
        audio_buf_size: u32,
        sample_rate: u32,
        min_depth: Duration,
        max_depth: Duration,
    ) -> io::Result<Self> {
        let data_socket = UdpSocket::bind(data_bind_addr).await?;
        let control_socket = UdpSocket::bind(control_bind_addr).await?;

        let local_data_addr = data_socket.local_addr()?;
        let local_control_addr = control_socket.local_addr()?;

        let pkt_buf = realtime::SharedPacketBuf::new(min_depth, max_depth);
        let is_alive = Arc::new(AtomicBool::new(true));
        // TODO : shutdown it!
        tokio::spawn(realtime::data_processor(
            data_socket,
            audio_buf_size,
            sample_rate,
            pkt_buf.clone(),
        ));
        tokio::spawn(realtime::control_processor(control_socket));

        Ok(Self {
            local_data_addr,
            local_control_addr,
            pkt_buf,
            is_alive,
        })
    }
}

impl BufferedChannel {
    pub async fn create(bind_addr: impl ToSocketAddrs, audio_buf_size: u32) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        let is_alive = Arc::new(AtomicBool::new(true));
        // TODO : shutdown
        tokio::spawn(async move {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    tracing::info!(%local_addr, %remote_addr, "accepting connection");
                    buffered::processor(stream, audio_buf_size).await;
                    tracing::info!(%local_addr, %remote_addr, "buffered stream done");
                }
                Err(err) => {
                    tracing::warn!(%err, %local_addr, "failed to accept connection");
                }
            }
        });

        Ok(Self {
            local_addr,
            audio_buf_size,
            is_alive,
        })
    }
}

impl DataChannel for Channel {
    type Content = AudioPacket;

    fn pull_data(&self) -> BufferedData<Self::Content> {
        match &self.inner {
            ChannelInner::Realtime(RealtimeChannel { pkt_buf, .. }) => {
                // TODO : channel may tear connection and it will spin-up indefintely
                let (data, wait_time) = pkt_buf.pop();
                BufferedData {
                    data: vec![],
                    wait_until_next: Some(wait_time),
                }
            }
            ChannelInner::Buffered(BufferedChannel { .. }) => BufferedData {
                data: vec![],
                wait_until_next: None,
            },
        }
    }
}

impl From<RealtimeChannel> for Channel {
    fn from(value: RealtimeChannel) -> Self {
        Self {
            inner: ChannelInner::Realtime(value),
        }
    }
}

impl From<BufferedChannel> for Channel {
    fn from(value: BufferedChannel) -> Self {
        Self {
            inner: ChannelInner::Buffered(value),
        }
    }
}

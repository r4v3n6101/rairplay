use std::{io, net::SocketAddr, sync::Arc};

use tokio::net::{TcpListener, ToSocketAddrs, UdpSocket};

use crate::{
    playback::{AudioStream, ChannelHandle},
    util::sync::CancellationHandle,
};

mod buffered;
mod packet;
mod realtime;

pub struct RealtimeChannel {
    pub local_data_addr: SocketAddr,
    pub local_control_addr: SocketAddr,
}

#[derive(Default)]
pub struct RealtimeSharedData {
    pub handle: CancellationHandle,
}

pub struct BufferedChannel {
    pub local_addr: SocketAddr,
    pub audio_buf_size: u32,
}

#[derive(Default)]
pub struct BufferedSharedData {
    pub handle: CancellationHandle,
}

impl RealtimeChannel {
    pub async fn create(
        data_bind_addr: impl ToSocketAddrs,
        control_bind_addr: impl ToSocketAddrs,
        audio_buf_size: u32,
        shared_data: Arc<RealtimeSharedData>,
        stream: impl AudioStream,
    ) -> io::Result<Self> {
        let data_socket = UdpSocket::bind(data_bind_addr).await?;
        let control_socket = UdpSocket::bind(control_bind_addr).await?;

        let local_data_addr = data_socket.local_addr()?;
        let local_control_addr = control_socket.local_addr()?;

        {
            let shared_data = shared_data.clone();
            tokio::spawn(async move {
                let task = async {
                    let data = realtime::data_processor(data_socket, audio_buf_size, &stream);
                    let control = realtime::control_processor(control_socket);

                    let (first, second) = tokio::join!(data, control);
                    first.or(second)
                };

                // tokio will handle it with boxing
                #[allow(clippy::large_futures)]
                match shared_data.handle.wrap_task(task).await {
                    Ok(()) => stream.on_ok(),
                    // TODO : error
                    Err(err) => stream.on_err(err.into()),
                }
            });
        }

        Ok(Self {
            local_data_addr,
            local_control_addr,
        })
    }
}

impl BufferedChannel {
    pub async fn create(
        bind_addr: impl ToSocketAddrs,
        audio_buf_size: u32,
        shared_data: Arc<BufferedSharedData>,
        stream: impl AudioStream,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        {
            let shared_data = shared_data.clone();
            tokio::spawn(async move {
                let task = async move {
                    match listener.accept().await {
                        Ok((stream, _)) => buffered::processor(stream, audio_buf_size).await,
                        Err(err) => Err(err),
                    }
                };

                match shared_data.handle.wrap_task(task).await {
                    Ok(()) => stream.on_ok(),
                    Err(err) => stream.on_err(err.into()),
                }
            });
        }

        Ok(Self {
            local_addr,
            audio_buf_size,
        })
    }
}

impl ChannelHandle for RealtimeSharedData {
    fn close(&self) {
        self.handle.close();
    }
}

impl ChannelHandle for BufferedSharedData {
    fn close(&self) {
        self.handle.close();
    }
}

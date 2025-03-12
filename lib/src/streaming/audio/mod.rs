use std::{io, net::SocketAddr, sync::Arc, time::Duration};

use tokio::net::{TcpListener, ToSocketAddrs, UdpSocket};

use crate::{
    device::{AudioPacket, BufferedData, DataChannel},
    util::sync::CancellationHandle,
};

mod buffered;
mod packet;
mod realtime;

pub struct RealtimeChannel {
    pub local_data_addr: SocketAddr,
    pub local_control_addr: SocketAddr,
    pub shared_data: Arc<RealtimeSharedData>,
}

pub struct RealtimeSharedData {
    pub handle: CancellationHandle<io::Result<()>>,
    pkt_buf: realtime::PacketBuf,
}

pub struct BufferedChannel {
    pub local_addr: SocketAddr,
    pub audio_buf_size: u32,
    pub shared_data: Arc<BufferedSharedData>,
}

pub struct BufferedSharedData {
    pub handle: CancellationHandle<io::Result<()>>,
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
        let shared_data = Arc::new(RealtimeSharedData {
            handle: CancellationHandle::default(),
            pkt_buf: realtime::PacketBuf::new(min_depth, max_depth),
        });

        {
            let shared_data = shared_data.clone();
            tokio::spawn(async move {
                let task = async {
                    let data = realtime::data_processor(
                        data_socket,
                        audio_buf_size,
                        sample_rate,
                        &shared_data.pkt_buf,
                    );
                    let control = realtime::control_processor(control_socket);

                    let (first, second) = tokio::join!(data, control);
                    first.or(second)
                };

                shared_data.handle.wrap_task(task).await;
            });
        }

        Ok(Self {
            local_data_addr,
            local_control_addr,
            shared_data,
        })
    }
}

impl BufferedChannel {
    pub async fn create(bind_addr: impl ToSocketAddrs, audio_buf_size: u32) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;
        let shared_data = Arc::new(BufferedSharedData {
            handle: CancellationHandle::default(),
        });

        {
            let shared_data = shared_data.clone();
            tokio::spawn(async move {
                let task = async move {
                    match listener.accept().await {
                        Ok((stream, _)) => buffered::processor(stream, audio_buf_size).await,
                        Err(err) => Err(err),
                    }
                };

                shared_data.handle.wrap_task(task).await;
            });
        }

        Ok(Self {
            local_addr,
            audio_buf_size,
            shared_data,
        })
    }
}

impl Drop for RealtimeChannel {
    fn drop(&mut self) {
        self.shared_data.handle.close();
    }
}

impl Drop for BufferedChannel {
    fn drop(&mut self) {
        self.shared_data.handle.close();
    }
}

impl DataChannel for RealtimeChannel {
    type Content = AudioPacket;

    fn pull_data(&self) -> BufferedData<Self::Content> {
        let (data, wait_time) = self.shared_data.pkt_buf.pop();
        BufferedData {
            data: vec![],
            wait_until_next: Some(wait_time),
        }
    }
}

impl DataChannel for BufferedChannel {
    type Content = AudioPacket;

    fn pull_data(&self) -> BufferedData<Self::Content> {
        BufferedData {
            data: vec![],
            wait_until_next: None,
        }
    }
}

use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::net::{TcpListener, ToSocketAddrs, UdpSocket};
use want::Taker;

use crate::{
    device::{AudioPacket, DataChannel, PullResult},
    util::sync::CancellationHandle,
};

mod buffered;
mod packet;
mod realtime;

pub struct RealtimeChannel {
    pub local_data_addr: SocketAddr,
    pub local_control_addr: SocketAddr,
    pub shared_data: Arc<RealtimeSharedData>,
    tk: Taker,
}

pub struct RealtimeSharedData {
    pub handle: CancellationHandle<io::Result<()>>,
    buffer: Mutex<Option<realtime::Data>>,
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
        let (gv, tk) = want::new();
        let shared_data = Arc::new(RealtimeSharedData {
            handle: CancellationHandle::default(),
            buffer: Mutex::new(Some(realtime::Data {
                // Wait a little before first data will be collected
                wait_time: min_depth,
                data: vec![],
            })),
        });

        {
            let shared_data = shared_data.clone();
            tokio::spawn(async move {
                let task = async {
                    let data = realtime::data_processor(
                        data_socket,
                        audio_buf_size,
                        sample_rate,
                        min_depth,
                        max_depth,
                        gv,
                        &shared_data.buffer,
                    );
                    let control = realtime::control_processor(control_socket);

                    let (first, second) = tokio::join!(data, control);
                    first.or(second)
                };

                // tokio will handle it with boxing
                #[allow(clippy::large_futures)]
                shared_data.handle.wrap_task(task).await;
            });
        }

        Ok(Self {
            local_data_addr,
            local_control_addr,
            shared_data,
            tk,
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

impl DataChannel for RealtimeChannel {
    type Content = AudioPacket;
    type Error<'a> = &'a io::Error;

    fn pull_data(&mut self) -> PullResult<Self::Content, Self::Error<'_>> {
        match self.shared_data.buffer.lock().unwrap().take() {
            Some(buf) => PullResult::Data {
                data: buf.data,
                wait_until_next: buf.wait_time,
            },
            None => match self.shared_data.handle.result() {
                Some(Ok(())) => PullResult::Finished,
                Some(Err(err)) => PullResult::Error(err),
                None => {
                    self.tk.want();
                    PullResult::Requested
                }
            },
        }
    }
}

impl DataChannel for BufferedChannel {
    type Content = AudioPacket;
    type Error<'a> = &'a io::Error;

    fn pull_data(&mut self) -> PullResult<Self::Content, Self::Error<'_>> {
        PullResult::Finished
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

use std::{io, net::SocketAddr, sync::Arc, time::Duration};

use tokio::net::{TcpListener, ToSocketAddrs};

use crate::{
    device::{DataChannel, PullResult, VideoPacket},
    util::sync::CancellationHandle,
};

mod buffered;
mod packet;

pub struct Channel {
    pub local_addr: SocketAddr,
    pub shared_data: Arc<SharedData>,
}

pub struct SharedData {
    pub handle: CancellationHandle<io::Result<()>>,
}

impl Channel {
    pub async fn create(
        bind_addr: impl ToSocketAddrs,
        video_buf_size: u32,
        latency: Duration,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;
        let shared_data = Arc::new(SharedData {
            handle: CancellationHandle::default(),
        });

        {
            let shared_data = shared_data.clone();
            tokio::spawn(async move {
                let task = async move {
                    match listener.accept().await {
                        Ok((stream, _)) => {
                            buffered::processor(stream, video_buf_size, latency).await
                        }
                        Err(err) => Err(err),
                    }
                };
                shared_data.handle.wrap_task(task).await;
            });
        }

        Ok(Self {
            local_addr,
            shared_data,
        })
    }
}

impl DataChannel for Channel {
    type Content = VideoPacket;
    type Error<'a> = ();

    fn pull_data(&mut self) -> PullResult<Self::Content, Self::Error<'_>> {
        PullResult::Finished
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        self.shared_data.handle.close();
    }
}

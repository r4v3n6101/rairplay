use std::{io, net::SocketAddr, sync::Arc, time::Duration};

use tokio::net::{TcpListener, ToSocketAddrs};

use crate::{
    device::{ChannelHandle, VideoStream},
    util::sync::CancellationHandle,
};

mod buffered;
mod packet;

pub struct Channel {
    pub local_addr: SocketAddr,
}

#[derive(Default)]
pub struct SharedData {
    pub handle: CancellationHandle,
}

impl Channel {
    pub async fn create(
        bind_addr: impl ToSocketAddrs,
        video_buf_size: u32,
        latency: Duration,
        shared_data: Arc<SharedData>,
        stream: impl VideoStream,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

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
                match shared_data.handle.wrap_task(task).await {
                    Ok(()) => stream.on_ok(),
                    Err(err) => stream.on_err(()),
                }
            });
        }

        Ok(Self { local_addr })
    }
}

impl ChannelHandle for SharedData {
    fn close(&self) {
        self.handle.close();
    }
}

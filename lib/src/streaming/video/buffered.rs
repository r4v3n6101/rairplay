use std::{io, net::SocketAddr};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use crate::device::{BufferedData, DataCallback};

use super::packet::VideoHeader;

pub struct Channel {
    local_addr: SocketAddr,
}

impl Channel {
    pub async fn create(bind_addr: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        tokio::spawn(async move {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    tracing::info!(%local_addr, %remote_addr, "accepting connection");
                    processor(stream).await;
                    tracing::info!(%local_addr, %remote_addr, "video stream done");
                }
                Err(err) => {
                    tracing::warn!(%err, %local_addr, "failed to accept connection");
                }
            }
        });

        Ok(Channel { local_addr })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn data_callback(&self) -> DataCallback<()> {
        // TODO
        Box::new(|| BufferedData {
            wait_until_next: None,
            data: Vec::new(),
        })
    }
}

async fn processor(mut stream: TcpStream) {
    loop {
        let mut header = VideoHeader::empty();
        if stream.read_exact(&mut *header).await.is_err() {
            break;
        }

        let mut payload = vec![0u8; header.payload_len() as usize];
        if stream.read_exact(&mut payload).await.is_err() {
            break;
        }

        // TODO : decrypt video
    }
}

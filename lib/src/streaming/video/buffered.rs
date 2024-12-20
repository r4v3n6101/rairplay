use std::{io, net::SocketAddr};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use crate::crypto::video::Cipher as VideoCipher;

use super::packet::VideoHeader;

async fn processor(mut stream: TcpStream, mut cipher: Option<VideoCipher>) {
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

pub struct Channel {
    local_addr: SocketAddr,
}

impl Channel {
    pub async fn create(
        bind_addr: impl ToSocketAddrs,
        cipher: Option<VideoCipher>,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        tokio::spawn(async move {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    tracing::info!(%local_addr, %remote_addr, "accepting connection");
                    processor(stream, cipher).await;
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
}

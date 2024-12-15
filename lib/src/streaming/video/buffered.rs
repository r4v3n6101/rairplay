use std::{io, net::SocketAddr};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use crate::crypto::video::AesCipher as AesVideoCipher;

use super::packet::VideoHeader;

async fn processor(mut stream: TcpStream, mut cipher: Option<AesVideoCipher>) {
    loop {
        let mut header = VideoHeader::empty();
        if let Err(err) = stream.read_exact(&mut *header).await {
            tracing::warn!(%err, "can't read video packet header");
            continue;
        }

        let mut payload = vec![0u8; header.payload_len() as usize];
        if let Err(err) = stream.read_exact(&mut payload).await {
            tracing::warn!(%err, "can't read video packet payload");
            continue;
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
        cipher: Option<AesVideoCipher>,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        tokio::spawn(async move {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    tracing::info!(%local_addr, %remote_addr, "accepting connection");
                    processor(stream, cipher).await;
                    // TODO : what if done with error?
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

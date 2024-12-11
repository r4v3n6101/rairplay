use std::{io, net::SocketAddr};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, ToSocketAddrs},
};

async fn processor(listener: TcpListener, local_addr: SocketAddr) {
    const BUF_SIZE: usize = 16 * 2024;

    let mut buf = [0; BUF_SIZE];
    while let Ok((mut stream, remote_addr)) = listener.accept().await {
        while let Ok(len @ 1..) = stream.read(&mut buf).await {
            tracing::debug!(%len, %remote_addr, %local_addr, "video data");
        }
    }
}

pub struct Channel {
    local_addr: SocketAddr,
}

impl Channel {
    pub async fn create(bind_addr: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        tokio::spawn(processor(listener, local_addr));

        Ok(Channel { local_addr })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

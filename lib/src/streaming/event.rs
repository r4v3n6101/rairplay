use std::{io, net::SocketAddr};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, ToSocketAddrs},
    sync::oneshot,
};

pub struct Channel {
    local_addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
}

impl Channel {
    pub async fn create(bind_addr: impl ToSocketAddrs) -> io::Result<Self> {
        const BUF_SIZE: usize = 16 * 2024;

        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;
        let (tx, rx) = oneshot::channel();
        let task = async move {
            let mut buf = [0; BUF_SIZE];
            while let Ok((mut stream, remote_addr)) = listener.accept().await {
                while let Ok(len @ 1..) = stream.read(&mut buf).await {
                    tracing::debug!(%len, %remote_addr, %local_addr, "event data");
                }
            }
        };

        tokio::spawn(async move {
            tokio::select! {
                _ = task => {}
                _ = rx => {}
            };
            tracing::info!("event listener done");
        });

        Ok(Channel {
            local_addr,
            shutdown: Some(tx),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        if let Some(Err(_)) | None = self.shutdown.take().map(|tx| tx.send(())) {
            tracing::warn!("event listener already closed");
        }
    }
}

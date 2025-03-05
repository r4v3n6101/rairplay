use std::{io, net::SocketAddr, sync::Arc};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, ToSocketAddrs},
    sync::Notify,
};

pub struct Channel {
    local_addr: SocketAddr,
    shutdown: Arc<Notify>,
}

impl Channel {
    pub async fn create(bind_addr: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;
        let notify = Arc::new(Notify::new());

        let notify1 = Arc::clone(&notify);
        tokio::spawn(async move {
            tokio::select! {
                () = notify1.notified() => {}
                () = processor(listener, local_addr) => {}
            };
            tracing::info!("event listener done");
        });

        Ok(Channel {
            local_addr,
            shutdown: notify,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        self.shutdown.notify_waiters();
    }
}

async fn processor(listener: TcpListener, local_addr: SocketAddr) {
    const BUF_SIZE: usize = 16 * 1024;

    let mut buf = [0; BUF_SIZE];
    while let Ok((mut stream, remote_addr)) = listener.accept().await {
        while let Ok(len @ 1..) = stream.read(&mut buf).await {
            tracing::debug!(%len, %remote_addr, %local_addr, "event data");
        }
    }
}

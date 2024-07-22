use std::io;

use tokio::net::{TcpListener, ToSocketAddrs};

use super::Handle;

// TODO : fix the errors inside, but it requires NTP client
// pub mod packet;

// TODO : this is a big logic of code and must be moved out w/ realtime one
pub async fn spawn(bind_addr: impl ToSocketAddrs) -> io::Result<Handle> {
    let listener = TcpListener::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        while let Ok((_, remote_addr)) = listener.accept().await {
            tracing::info!(%remote_addr, "buffered got a new one");
            loop {}
        }
    })
    .abort_handle();

    Ok(Handle { handle, local_addr })
}

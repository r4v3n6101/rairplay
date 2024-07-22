use std::io;

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, ToSocketAddrs},
};

use super::Handle;

pub async fn spawn_tracing(bind_addr: impl ToSocketAddrs) -> io::Result<Handle> {
    const BUF_SIZE: usize = 16 * 1024;

    let listener = TcpListener::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        let mut buf = [0; BUF_SIZE];
        while let Ok((mut stream, remote_addr)) = listener.accept().await {
            while let Ok(len @ 1..) = stream.read(&mut buf).await {
                tracing::debug!(%len, %remote_addr, %local_addr, "get some data at event channel" );
            }
        }
    })
    .abort_handle();

    Ok(Handle { handle, local_addr })
}

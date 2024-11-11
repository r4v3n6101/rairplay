use std::net::SocketAddr;

use tokio::task::AbortHandle;

pub mod audio;
pub mod event;
pub mod video;

// TODO : get rid of it
pub struct Handle {
    /// Used for cancelling task with processing data
    pub(self) handle: AbortHandle,
    /// Usually only port is required
    pub(self) local_addr: SocketAddr,
}

impl Handle {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        tracing::info!(local_addr=%self.local_addr, "closing channel");
        self.handle.abort();
    }
}

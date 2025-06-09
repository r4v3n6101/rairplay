use std::future::Future;

use tokio::sync::Semaphore;

pub struct CancellationHandle {
    sem: Semaphore,
}

impl Default for CancellationHandle {
    fn default() -> Self {
        Self {
            sem: Semaphore::new(0),
        }
    }
}

impl CancellationHandle {
    pub async fn wrap_task<F, Err>(&self, task: F) -> Result<(), Err>
    where
        F: Future<Output = Result<(), Err>>,
    {
        let res = tokio::select! {
            res = task => res,
            _ = self.sem.acquire() => Ok(()),
        };
        self.sem.close();
        res
    }

    pub fn close(&self) {
        if !self.sem.is_closed() {
            self.sem.add_permits(1);
        }
    }
}

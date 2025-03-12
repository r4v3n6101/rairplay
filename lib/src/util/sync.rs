use std::{cell::UnsafeCell, future::Future, mem::MaybeUninit};

use tokio::sync::Semaphore;

pub struct CancellationHandle<T> {
    sem: Semaphore,
    res: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Send for CancellationHandle<T> {}
unsafe impl<T> Sync for CancellationHandle<T> {}

impl<T> Default for CancellationHandle<T> {
    fn default() -> Self {
        Self {
            sem: Semaphore::new(0),
            res: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
}

impl<T> CancellationHandle<T> {
    pub fn close(&self) {
        if !self.sem.is_closed() {
            self.sem.add_permits(1);
        }
    }

    pub fn result(&self) -> Option<&T> {
        if self.sem.is_closed() {
            Some(unsafe { (*self.res.get()).assume_init_ref() })
        } else {
            None
        }
    }
}

impl<E> CancellationHandle<Result<(), E>> {
    pub async fn wrap_task<F>(&self, task: F)
    where
        F: Future<Output = Result<(), E>>,
    {
        // Safety: no one calls get until semaphore is closed what happened only after set
        tokio::select! {
            res = task => unsafe {
                *self.res.get() = MaybeUninit::new(res);
            },
            _ = self.sem.acquire() => unsafe {
                *self.res.get() = MaybeUninit::new(Ok(()));
            },
        }
        self.sem.close();
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::CancellationHandle;

    #[tokio::test]
    async fn test_task_failure() {
        let handle: Arc<CancellationHandle<Result<(), u32>>> = Arc::default();
        let task = async { Err(666) };

        handle.wrap_task(task).await;

        assert_eq!(666, handle.result().unwrap().unwrap_err());
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let handle: Arc<CancellationHandle<Result<(), u32>>> = Arc::default();
        let task = async {
            // Emulate some work so we can cancel it
            tokio::time::sleep(Duration::from_millis(500)).await;
            Err(666)
        };

        {
            let handle = handle.clone();
            tokio::spawn(async move { handle.wrap_task(task).await });
        }
        {
            let handle = handle.clone();
            tokio::spawn(async move {
                // Sleep less than task, so we're not late to cancel it
                tokio::time::sleep(Duration::from_millis(100)).await;
                handle.close();
            });
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(Ok::<_, u32>(()), *handle.result().unwrap());
    }

    #[tokio::test]
    async fn test_task_finish() {
        let handle: Arc<CancellationHandle<Result<(), u32>>> = Arc::default();
        let task = async {
            // Sleep a little, so we can return before the task got cancelled
            tokio::time::sleep(Duration::from_millis(100)).await;
            Err(0u32)
        };

        {
            let handle = handle.clone();
            tokio::spawn(async move {
                handle.wrap_task(task).await;
            });
        }
        {
            let handle = handle.clone();
            tokio::spawn(async move {
                // Must be late
                tokio::time::sleep(Duration::from_millis(500)).await;
                handle.close();
            });
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(Err(0u32), *handle.result().unwrap());
    }
}

use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};

use futures::task::AtomicWaker;

pub struct WakerFlag {
    waker: AtomicWaker,
    flag: AtomicBool,
}

impl Default for WakerFlag {
    fn default() -> Self {
        Self {
            waker: AtomicWaker::new(),
            flag: AtomicBool::new(false),
        }
    }
}

impl WakerFlag {
    pub fn set_and_wake(&self) {
        self.flag.store(true, Ordering::Release);
        self.waker.wake();
    }
}

// We've got this behind ref
impl Future for &WakerFlag {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.flag.load(Ordering::Acquire) {
            return Poll::Ready(());
        }
        self.waker.register(cx.waker());
        if self.flag.load(Ordering::Acquire) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

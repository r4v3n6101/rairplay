use std::sync::{Arc, RwLock};

use crossbeam_utils::atomic::AtomicCell;
use futures::stream::AbortHandle;

use crate::adv::Advertisment;

use super::dto::{SenderInfo, StreamDescriptor, StreamInfo};

#[derive(Debug, Clone)]
pub struct SharedState {
    pub state: Arc<State>,
    pub adv: Arc<Advertisment>,
}

#[derive(Debug, Default)]
pub struct State {
    pub sender: RwLock<Option<SenderHandle>>,
    pub streams: RwLock<Vec<StreamHandle>>,

    pub volume: AtomicCell<f32>,
    pub playback_rate: AtomicCell<f32>,
    pub progress: AtomicCell<(u32, u32, u32)>,
}

#[derive(Debug)]
pub struct SenderHandle {
    pub info: SenderInfo,
    pub event_handle: AbortHandle,
}

impl Drop for SenderHandle {
    fn drop(&mut self) {
        self.event_handle.abort()
    }
}

#[derive(Debug)]
pub struct StreamHandle {
    pub info: StreamInfo,
    pub descriptor: StreamDescriptor,
    pub data_handle: AbortHandle,
    pub control_handle: AbortHandle,
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        self.data_handle.abort();
        self.control_handle.abort();
    }
}

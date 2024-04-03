use std::sync::{Arc, RwLock};

use crossbeam_utils::atomic::AtomicCell;
use mac_address::MacAddress;
use tokio_util::sync::{CancellationToken, DropGuard};

#[derive(Debug, Default, Clone)]
pub struct SharedState(pub Arc<State>);

#[derive(Debug, Default)]
pub struct State {
    pub sender_info: RwLock<Option<SenderInfo>>,
    pub streams: RwLock<Vec<Stream>>,

    pub volume: AtomicCell<f32>,
    pub playback_rate: AtomicCell<f32>,
    pub progress: AtomicCell<(u32, u32, u32)>,
}

#[derive(Debug)]
pub struct SenderInfo {
    pub device_id: String,
    pub name: String,
    pub model: String,
    pub mac_address: MacAddress,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_build_version: Option<String>,

    pub timing_proto: TimingProtocol,

    pub cancellation_token: CancellationToken,
}

impl Drop for SenderInfo {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
    }
}

#[derive(Debug)]
pub enum TimingProtocol {
    Ptp,
    Ntp,
}

#[derive(Debug)]
pub struct Stream {
    pub id: u32,
    pub ty: StreamType,
    pub client_id: Option<String>,
    pub metadata: StreamMetadata,

    pub cancellation_token: CancellationToken,
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.cancellation_token.cancel()
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum StreamType {
    AudioRealTime = 96,
    AudioBuffered = 103,
    // Screen (110)
    // Playback (120)
    // RemoteControl (130)
}

#[derive(Debug)]
pub enum StreamMetadata {
    Audio {
        audio_buffer_size: u32,
        latency_min: Option<u32>,
        latency_max: Option<u32>,
    },
}

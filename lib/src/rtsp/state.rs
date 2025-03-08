use std::{
    collections::BTreeMap,
    ops::Deref,
    sync::{atomic::AtomicU64, Arc, Mutex},
};

use bytes::Bytes;
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    crypto::pairing::legacy::State as LegacyPairing,
    info::Config,
    streaming::{self, event::Channel as EventChannel},
};

pub struct State {
    pub last_stream_id: AtomicU64,
    pub pairing: Mutex<LegacyPairing>,
    pub fp_last_msg: Mutex<Bytes>,
    pub fp_key: Mutex<Bytes>,
    pub event_channel: AsyncMutex<Option<EventChannel>>,
    pub audio_realtime_channels: Mutex<BTreeMap<u64, streaming::audio::RealtimeChannel>>,
    pub audio_buffered_channels: Mutex<BTreeMap<u64, streaming::audio::BufferedChannel>>,
    pub video_channels: Mutex<BTreeMap<u64, streaming::video::ChannelInner>>,

    pub cfg: Config,
}

#[derive(Clone)]
pub struct SharedState(Arc<State>);

impl Deref for SharedState {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SharedState {
    pub fn with_config(cfg: Config) -> Self {
        Self(Arc::new(State {
            last_stream_id: AtomicU64::default(),
            pairing: Mutex::new(LegacyPairing::from_signing_privkey(
                cfg.pairing.legacy_pairing_key,
            )),
            fp_last_msg: Mutex::default(),
            fp_key: Mutex::default(),
            event_channel: AsyncMutex::default(),
            audio_realtime_channels: Mutex::default(),
            audio_buffered_channels: Mutex::default(),
            video_channels: Mutex::default(),

            cfg,
        }))
    }
}

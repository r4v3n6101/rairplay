use std::{
    ops::Deref,
    sync::{atomic::AtomicU64, Arc, Mutex, Weak},
};

use bytes::Bytes;
use tokio::sync::Mutex as AsyncMutex;
use weak_table::WeakValueHashMap;

use crate::{
    config::Config,
    crypto::pairing::legacy::State as LegacyPairing,
    streaming::{EventChannel, SharedData},
};

pub struct State<ADev, VDev> {
    pub last_stream_id: AtomicU64,
    pub pairing: Mutex<LegacyPairing>,
    pub fp_last_msg: Mutex<Bytes>,
    pub ekey: Mutex<Bytes>,
    pub eiv: Mutex<Bytes>,
    pub event_channel: AsyncMutex<Option<EventChannel>>,
    pub audio_realtime_channels: Mutex<WeakValueHashMap<u64, Weak<SharedData>>>,
    pub audio_buffered_channels: Mutex<WeakValueHashMap<u64, Weak<SharedData>>>,
    pub video_channels: Mutex<WeakValueHashMap<u64, Weak<SharedData>>>,

    pub cfg: Config<ADev, VDev>,
}

pub struct SharedState<ADev, VDev>(Arc<State<ADev, VDev>>);

impl<A, V> Clone for SharedState<A, V> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<A, V> Deref for SharedState<A, V> {
    type Target = State<A, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A, V> SharedState<A, V> {
    pub fn with_config(cfg: Config<A, V>) -> Self {
        Self(Arc::new(State {
            last_stream_id: AtomicU64::default(),
            pairing: Mutex::new(LegacyPairing::from_signing_privkey(
                cfg.pairing.legacy_pairing_key,
            )),
            fp_last_msg: Mutex::default(),
            ekey: Mutex::default(),
            eiv: Mutex::default(),
            event_channel: AsyncMutex::default(),
            audio_realtime_channels: Mutex::default(),
            audio_buffered_channels: Mutex::default(),
            video_channels: Mutex::default(),

            cfg,
        }))
    }
}

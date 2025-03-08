use std::{
    collections::BTreeMap,
    ops::Deref,
    sync::{atomic::AtomicU64, Arc, Mutex},
};

use bytes::Bytes;
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    crypto::pairing::legacy::State as LegacyPairing, device::StreamHandle, info::Config,
    streaming::event::Channel as EventChannel,
};

pub struct State {
    pub cfg: Config,
    pub last_stream_id: AtomicU64,

    pub pairing: Mutex<LegacyPairing>,
    pub fp_last_msg: Mutex<Bytes>,
    pub fp_key: Mutex<Bytes>,

    pub event_channel: AsyncMutex<Option<EventChannel>>,

    pub stream_handles: Mutex<BTreeMap<StreamDescriptor, Box<dyn StreamHandle>>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StreamDescriptor {
    pub id: u64,
    pub ty: u32,
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
            cfg,
            last_stream_id: AtomicU64::default(),

            // TODO : change this shit
            pairing: Mutex::new(LegacyPairing::from_signing_privkey([5; 32])),
            fp_last_msg: Mutex::default(),
            fp_key: Mutex::default(),

            event_channel: AsyncMutex::default(),

            stream_handles: Mutex::default(),
        }))
    }
}

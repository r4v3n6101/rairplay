use std::{
    ops::Deref,
    sync::{atomic::AtomicU32, Arc, Mutex},
};

use bytes::Bytes;
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    crypto::pairing::legacy::State as LegacyPairing,
    info::Config,
    streaming::{command::Dispatcher as CmdDispatcher, event::Channel as EventChannel},
};

#[derive(Clone)]
pub struct SharedState(pub Arc<State>);

pub struct State {
    pub cfg: Config,
    pub last_stream_id: AtomicU32,

    pub pairing: Mutex<LegacyPairing>,
    pub fp_last_msg: Mutex<Bytes>,
    pub fp_key: Mutex<Bytes>,

    pub event_channel: AsyncMutex<Option<EventChannel>>,
    pub cmd_channel: CmdDispatcher,
}

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
            last_stream_id: AtomicU32::default(),

            // TODO : change this shit
            pairing: Mutex::new(LegacyPairing::from_signing_privkey([5; 32])),
            fp_last_msg: Mutex::default(),
            fp_key: Mutex::default(),

            event_channel: AsyncMutex::default(),
            cmd_channel: CmdDispatcher::default(),
        }))
    }
}

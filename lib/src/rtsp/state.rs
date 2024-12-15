use std::{
    ops::Deref,
    sync::{atomic::AtomicU32, Arc, Mutex},
};

use bytes::Bytes;
use tokio::sync::Mutex as AsyncMutex;

use crate::{info::Config, streaming};

#[derive(Clone)]
pub struct SharedState(pub Arc<State>);

pub struct State {
    pub cfg: Config,
    pub last_stream_id: AtomicU32,

    pub fp_msg3: Mutex<Option<Bytes>>,

    pub event_channel: AsyncMutex<Option<streaming::event::Channel>>,
    pub cmd_channel: streaming::command::Dispatcher,
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
            last_stream_id: Default::default(),

            fp_msg3: Default::default(),

            event_channel: Default::default(),
            cmd_channel: Default::default(),
        }))
    }
}

use std::{
    ops::Deref,
    sync::{atomic::AtomicU32, Arc},
};

use tokio::sync::Mutex as AsyncMutex;

use crate::{info::Config, streaming};

#[derive(Clone)]
pub struct SharedState(pub Arc<State>);

impl Deref for SharedState {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct State {
    pub cfg: Config,
    pub last_stream_id: AtomicU32,
    pub event_channel: AsyncMutex<Option<streaming::event::Channel>>,
    pub cmd_channel: streaming::command::Dispatcher,
}

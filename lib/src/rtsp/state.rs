use std::{
    ops::Deref,
    sync::{Arc, Mutex},
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
    pub event_channel: AsyncMutex<Option<streaming::event::Channel>>,
    pub dispatchers: Mutex<Vec<streaming::command::Dispatcher>>,
}

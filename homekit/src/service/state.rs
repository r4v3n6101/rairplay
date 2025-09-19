use std::{ops::Deref, sync::Arc};

use crate::config::PinCode;

pub struct State {
    pub pin: PinCode,
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
    pub fn new(pin: PinCode) -> Self {
        Self(Arc::new(State { pin }))
    }
}

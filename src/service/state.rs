use std::sync::Arc;

use crate::adv::Advertisment;

#[derive(Debug, Clone)]
pub struct SharedState {
    pub advertisment: Arc<Advertisment>,
    pub state: Arc<State>,
}

#[derive(Debug)]
pub struct State {}

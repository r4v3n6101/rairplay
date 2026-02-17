use std::sync::Mutex;

use super::handlers::{setup::State as SetupState, verify::State as VerifyState};
use crate::config::PinCode;

pub struct ServiceState {
    pub setup_state: Mutex<SetupState>,
    pub verify_state: Mutex<VerifyState>,
}

impl ServiceState {
    pub fn new(pin: Option<PinCode>) -> Self {
        Self {
            setup_state: Mutex::new(SetupState::new(pin)),
            verify_state: Mutex::new(VerifyState::new()),
        }
    }
}

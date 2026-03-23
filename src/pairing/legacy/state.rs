use std::sync::Mutex;

use super::handlers::inner::State as InnerState;

pub struct ServiceState {
    pub pairing: Mutex<InnerState>,
}

impl ServiceState {
    pub fn new(privkey: &[u8]) -> Self {
        Self {
            pairing: Mutex::new(InnerState::from_signing_privkey(privkey)),
        }
    }
}

use std::sync::Mutex;

use super::handlers::inner::{Ed25519Key, State as InnerState};

// TODO : better naming
pub struct ServiceState {
    pub pairing: Mutex<InnerState>,
}

impl ServiceState {
    pub fn new(privkey: Ed25519Key) -> Self {
        Self {
            pairing: Mutex::new(InnerState::from_signing_privkey(privkey)),
        }
    }
}

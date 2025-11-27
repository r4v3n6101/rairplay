use std::sync::Mutex;

use crate::crypto::Ed25519Key;

use super::handlers::inner::State as InnerState;

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

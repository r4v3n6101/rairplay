use std::sync::Mutex;

use super::handlers::inner::State as InnerState;
use crate::crypto::Ed25519Key;

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

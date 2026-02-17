use std::sync::Arc;

use tokio::sync::Mutex as AsyncMutex;

use super::{
    super::KeychainHolder,
    handlers::{setup::State as SetupState, verify::State as VerifyState},
};
use crate::config::PinCode;

pub struct ServiceState<K> {
    pub keychain_holder: Arc<dyn KeychainHolder<Keychain = K>>,
    pub setup_state: AsyncMutex<SetupState>,
    pub verify_state: AsyncMutex<VerifyState>,
}

impl<K> ServiceState<K> {
    pub fn new(
        keychain_holder: Arc<dyn KeychainHolder<Keychain = K>>,
        pin: Option<PinCode>,
    ) -> Self {
        Self {
            keychain_holder,
            setup_state: AsyncMutex::new(SetupState::new(pin)),
            verify_state: AsyncMutex::new(VerifyState::new()),
        }
    }
}

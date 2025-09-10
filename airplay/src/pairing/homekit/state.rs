use tokio::sync::Mutex as AsyncMutex;

use crate::config::PinCode;

use super::handlers::setup::State as SetupState;

pub struct ServiceState {
    pub setup_state: AsyncMutex<SetupState>,
    // TODO
    pub verify_state: AsyncMutex<()>,
}

impl ServiceState {
    pub fn new(pin: Option<PinCode>) -> Self {
        Self {
            setup_state: AsyncMutex::new(SetupState::new(pin)),
            verify_state: AsyncMutex::new(()),
        }
    }
}

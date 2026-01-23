use std::sync::Arc;

use axum::{Router, routing::post};

use crate::config::{Keychain, PinCode};

use super::KeychainHolder;

mod dto;
mod extractor;
mod handlers;
mod state;
mod transport;

pub fn router<K>(
    keychain_holder: Arc<dyn KeychainHolder<Keychain = K>>,
    pin: Option<PinCode>,
) -> Router<()>
where
    K: Keychain,
{
    let state = Arc::new(state::ServiceState::new(keychain_holder, pin));
    Router::new()
        .route("/pair-setup", post(handlers::pair_setup))
        // .route("/pair-verify", post(()))
        // .route("/pair-list", post(()))
        // .route("/pair-add", post(()))
        // .route("/pair-pin-start", post(()))
        .with_state(state)
}

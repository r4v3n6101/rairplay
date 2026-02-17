use std::sync::Arc;

use axum::{Extension, Router, routing::post};

use super::{KeychainHolder, SessionKeyHolder};
use crate::config::{Keychain, PinCode};

mod dto;
mod extractor;
mod handlers;
mod state;
mod transport;

pub fn router<K>(
    keychain_holder: Arc<dyn KeychainHolder<Keychain = K>>,
    key_holder: Arc<dyn SessionKeyHolder>,
    pin: Option<PinCode>,
) -> Router<()>
where
    K: Keychain,
{
    let state = Arc::new(state::ServiceState::new(pin));
    Router::new()
        .route("/pair-setup", post(handlers::pair_setup::<K>))
        .route("/pair-verify", post(handlers::pair_verify::<K>))
        // .route("/pair-list", post(()))
        // .route("/pair-add", post(()))
        // .route("/pair-pin-start", post(()))
        .with_state(state)
        .layer(Extension(keychain_holder))
        .layer(Extension(key_holder))
}

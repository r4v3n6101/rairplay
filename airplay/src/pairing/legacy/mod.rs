use std::sync::Arc;

use axum::{Extension, Router, routing::post};

use super::{KeychainHolder, SessionKeyHolder};
use crate::config::Keychain;

mod handlers;
mod state;

pub fn router<K>(
    keychain_holder: &dyn KeychainHolder<Keychain = K>,
    key_holder: Arc<dyn SessionKeyHolder>,
) -> Router<()>
where
    K: Keychain,
{
    Router::new()
        .route("/pair-setup", post(handlers::pair_setup))
        .route("/pair-verify", post(handlers::pair_verify))
        .with_state(Arc::new(state::ServiceState::new(
            keychain_holder
                .keychain()
                .pubkey()
                .try_into()
                .expect("valid ed25519 key"),
        )))
        .layer(Extension(key_holder))
}

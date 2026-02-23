use std::sync::Arc;

use axum::{Extension, Router, routing::post};
use yoke::{Yoke, erased::ErasedArcCart};

use super::SharedSessionKey;
use crate::config::{Keychain, PinCode};

pub mod codec;

mod dto;
mod extractor;
mod handlers;
mod state;

pub fn router<K>(
    keychain: Yoke<&'static K, ErasedArcCart>,
    session_key: SharedSessionKey,
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
        .layer(Extension(keychain))
        .layer(Extension(session_key))
}

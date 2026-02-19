use std::sync::Arc;

use axum::{Extension, Router, routing::post};
use seqlock::SeqLock;
use yoke::{Yoke, erased::ErasedArcCart};

use crate::{config::Keychain, crypto::SessionKey};

mod handlers;
mod state;

pub fn router<K>(
    keychain: Yoke<&'static K, ErasedArcCart>,
    session_key: Yoke<&'static SeqLock<Option<SessionKey>>, ErasedArcCart>,
) -> Router<()>
where
    K: Keychain,
{
    Router::new()
        .route("/pair-setup", post(handlers::pair_setup))
        .route("/pair-verify", post(handlers::pair_verify))
        .with_state(Arc::new(state::ServiceState::new(keychain.get().pubkey())))
        .layer(Extension(session_key))
}

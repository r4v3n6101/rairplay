use std::sync::Arc;

use axum::{Extension, Router, routing::post};

use crate::crypto::Ed25519Key;

use super::SessionKeyHolder;

mod handlers;
mod state;

pub fn router(holder: Arc<dyn SessionKeyHolder>, privkey: Ed25519Key) -> Router<()> {
    let state = Arc::new(state::ServiceState::new(privkey));
    Router::new()
        .route("/pair-setup", post(handlers::pair_setup))
        .route("/pair-verify", post(handlers::pair_verify))
        .with_state(state)
        .layer(Extension(holder))
}

use std::sync::Arc;

use axum::{Router, routing::post};

use crate::config::PinCode;

mod dto;
mod extractor;
mod handlers;
mod state;
mod transport;

pub fn router(pin: Option<PinCode>) -> Router<()> {
    let state = Arc::new(state::ServiceState::new(pin));

    Router::new()
        .route("/pair-setup", post(handlers::pair_setup))
        // .route("/pair-verify", post(()))
        // .route("/pair-list", post(()))
        // .route("/pair-add", post(()))
        // .route("/pair-pin-start", post(()))
        .with_state(state)
}

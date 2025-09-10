use std::sync::Arc;

use axum::{Extension, Router, routing::post};

use super::SessionKeyHolder;

mod handlers;
mod state;

// TODO : replace privkey with type
pub fn router(holder: Arc<dyn SessionKeyHolder>, privkey: [u8; 32]) -> Router<()> {
    let state = Arc::new(state::ServiceState::new(privkey));
    Router::new()
        .route("/pair-setup", post(handlers::pair_setup))
        .route("/pair-verify", post(handlers::pair_verify))
        .with_state(state)
        .layer(Extension(holder))
}

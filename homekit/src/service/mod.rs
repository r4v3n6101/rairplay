use axum::{routing::post, Router};

use state::SharedState;

use crate::config::PinCode;

mod dto;
mod extractor;
mod handlers;
mod state;

pub struct HomeKitService {
    inner: Router<SharedState>,
}

impl HomeKitService {
    // TODO : login attempts, DB with cached logins and keys
    pub fn new(pin: PinCode) -> Self {
        let state = SharedState::new(pin);
        let inner = Router::<SharedState>::new()
            .route("/pair-setup", post(handlers::pair_setup))
            .route("/pair-verify", post(handlers::pair_verify))
            // TODO : pair_list, pair_add, etc.
            .with_state(state);

        Self { inner }
    }
}

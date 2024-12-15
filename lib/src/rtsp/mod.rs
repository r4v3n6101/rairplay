use axum::{
    extract::Request,
    handler::Handler,
    http::HeaderName,
    routing::{any, get, post},
    Router,
};
use state::SharedState;
use tower_http::propagate_header::PropagateHeaderLayer;

use crate::info::Config;

mod dto;
mod handlers;
mod plist;
mod state;

// TODO : make this MakeService and stop sharing same state across all clients because it's a bug
pub fn svc_router(cfg: Config) -> Router<()> {
    let state = SharedState::with_config(cfg);
    Router::new()
        // Heartbeat
        .route("/feedback", post(()))
        // I guess it will never be used
        .route("/command", post(()))
        // General info about server
        .route("/info", get(handlers::info))
        // Legacy pairing (shared secret via ecdh)
        .route("/pair-setup", post(handlers::pair_setup))
        .route("/pair-verify", post(handlers::pair_verify))
        // Fair play, but seems like it isn't working correct
        .route("/fp-setup", post(handlers::fp_setup))
        // Unknown handlers, just trace response
        .fallback(handlers::generic)
        // State cloned here, because it will be moved below
        .with_state(state.clone())
        // Custom RTSP methods
        .route(
            "/:media_id",
            any(|req: Request| async move {
                match req.method().as_str() {
                    "SETUP" => handlers::setup.call(req, state).await,
                    "GET_PARAMETER" => handlers::get_parameter.call(req, state).await,
                    "FLUSHBUFFERED" => handlers::flush_buffered.call(req, state).await,
                    "SETRATEANCHORTIME" => handlers::set_rate_anchor_time.call(req, state).await,
                    // TODO : impl empty handlers
                    //"SET_PARAMETER" => set_parameter::handler.call(req, state).await,
                    //"TEARDOWN" => handlers::teardown.call(req, ()).await,
                    method => {
                        tracing::error!(?method, path = ?req.uri(), "unknown method");
                        handlers::generic.call(req, state).await
                    }
                }
            }),
        )
        // CSeq is required for RTSP protocol
        .layer(PropagateHeaderLayer::new(HeaderName::from_static("cseq")))
}

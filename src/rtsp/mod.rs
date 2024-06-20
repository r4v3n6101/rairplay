use std::sync::Arc;

use axum::{
    extract::Request,
    handler::Handler,
    http::{HeaderName, HeaderValue},
    routing::{any, get, post},
    Router,
};
use tower_http::{propagate_header::PropagateHeaderLayer, set_header::SetResponseHeaderLayer};

use crate::{adv::Advertisment, rtsp::state::SharedState};

mod dto;
mod handlers;
mod rtp;
mod state;

pub fn route(adv: Arc<Advertisment>) -> Router<()> {
    let state = SharedState {
        state: Arc::default(),
        adv,
    };

    Router::new()
        // Heartbeat
        .route("/feedback", post(()))
        // I guess it will never be used
        .route("/command", post(()))
        // General info about server
        .route("/info", get(handlers::info))
        // Fair play, but seems like it isn't working correct
        .route("/fp-setup", post(handlers::fp_setup))
        .with_state(state.clone())
        // Custom RTSP methods
        .route(
            "/:media_id",
            any(|req: Request| async move {
                // TODO : impl empty handlers
                match req.method().as_str() {
                    "SETUP" => handlers::setup.call(req, state).await,
                    // Get parameters such as volume
                    "GET_PARAMETER" => handlers::get_parameter.call(req, state).await,
                    // Set parameters such as artwork, volume, position
                    //"SET_PARAMETER" => set_parameter::handler.call(req, state).await,
                    "SETRATEANCHORTIME" => handlers::setrateanchortime.call(req, state).await,
                    // Remove stream handle and closes all its channels
                    "TEARDOWN" => handlers::teardown.call(req, state).await,
                    // Flush remained data, called before teardown
                    "FLUSHBUFFERED" => handlers::flushbuffered.call(req, state).await,
                    method => {
                        tracing::error!(?method, path = ?req.uri(), "unknown method");
                        handlers::trace_body.call(req, state).await
                    }
                }
            }),
        )
        // Unknown handlers, just trace response
        .fallback(handlers::trace_body)
        // CSeq is required for RTSP protocol
        .layer(PropagateHeaderLayer::new(HeaderName::from_static("cseq")))
        // Synthetic header to let mapper know that's RTSP, not HTTP
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("upgrade"),
            |_: &_| Some(HeaderValue::from_static("RTSP")),
        ))
}

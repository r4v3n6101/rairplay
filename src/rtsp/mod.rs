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

// TODO : move out to handlers
mod flush;
mod fp_setup;
mod generic;
mod get_parameter;
mod info;
mod set_parameter;
mod setrateanchortime;
mod setup;
mod teardown;

mod dto;
mod rtp;
mod state;

pub fn route(adv: Arc<Advertisment>) -> Router<()> {
    let state = SharedState {
        state: Arc::default(),
        adv,
    };

    Router::new()
        // Heart beat
        .route("/feedback", post(()))
        // I guess it will never be used
        .route("/command", post(()))
        // General info about server
        .route("/info", get(info::handler))
        // Fair play, but seems like it isn't working correct
        .route("/fp-setup", post(fp_setup::handler))
        .with_state(state.clone())
        // Custom RTSP methods
        .route(
            "/:media_id",
            any(|req: Request| async move {
                // TODO : impl empty handlers
                match req.method().as_str() {
                    "SETUP" => setup::handler.call(req, state).await,
                    // Get parameters such as volume
                    "GET_PARAMETER" => get_parameter::handler.call(req, state).await,
                    // Set parameters such as artwork, volume, position
                    "SET_PARAMETER" => set_parameter::handler.call(req, state).await,
                    "SETRATEANCHORTIME" => setrateanchortime::handler.call(req, state).await,
                    // Remove stream handle and closes all its channels
                    "TEARDOWN" => teardown::handler.call(req, state).await,
                    // Flush remained data, called before teardown
                    "FLUSHBUFFERED" => flush::handle_buffered.call(req, state).await,
                    method => {
                        tracing::error!(?method, path = ?req.uri(), "unknown method");
                        generic::trace_body.call(req, ()).await
                    }
                }
            }),
        )
        // Unknown handlers, just trace response
        .fallback(generic::trace_body)
        // CSeq is required for RTSP protocol
        .layer(PropagateHeaderLayer::new(HeaderName::from_static("cseq")))
        // Synthetic header to let mapper know that's RTSP, not HTTP
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("upgrade"),
            |_: &_| Some(HeaderValue::from_static("RTSP")),
        ))
}

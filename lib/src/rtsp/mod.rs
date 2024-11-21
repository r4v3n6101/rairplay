use std::sync::Arc;

use axum::{
    extract::Request,
    handler::Handler,
    http::{HeaderName, HeaderValue},
    routing::{any, get, post},
    Router,
};
use state::{SharedState, State};
use tokio::sync::Mutex;
use tower_http::{propagate_header::PropagateHeaderLayer, set_header::SetResponseHeaderLayer};

use crate::info::Config;

mod dto;
mod handlers;
mod plist;
mod state;

pub fn svc_router(cfg: Config) -> Router<()> {
    let state = State {
        cfg,
        event_channel: Mutex::new(None),
    };
    let state = SharedState(Arc::new(state));
    Router::new()
        // Heartbeat
        .route("/feedback", post(()))
        // I guess it will never be used
        .route("/command", post(()))
        // General info about server
        .route("/info", get(handlers::info))
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

                    // TODO : impl empty handlers
                    //"SET_PARAMETER" => set_parameter::handler.call(req, state).await,
                    //"SETRATEANCHORTIME" => handlers::set_rate_anchor_time.call(req, state).await,
                    //"TEARDOWN" => handlers::teardown.call(req, ()).await,
                    //"FLUSHBUFFERED" => handlers::flush_buffered.call(req, state).await,
                    method => {
                        tracing::error!(?method, path = ?req.uri(), "unknown method");
                        handlers::generic.call(req, state).await
                    }
                }
            }),
        )
        // CSeq is required for RTSP protocol
        .layer(PropagateHeaderLayer::new(HeaderName::from_static("cseq")))
        // Synthetic header to let mapper know that's RTSP, not HTTP
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("upgrade"),
            |_: &_| Some(HeaderValue::from_static("RTSP")),
        ))
}

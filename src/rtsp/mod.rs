use axum::{
    extract::Request,
    handler::Handler,
    http::{HeaderName, HeaderValue},
    routing::{any, get, post},
    Router,
};
use tower_http::{propagate_header::PropagateHeaderLayer, set_header::SetResponseHeaderLayer};

use crate::rtsp::state::SharedState;

mod fp_setup;
mod generic;
mod get_parameter;
mod info;
mod setup;

mod plist;
mod state;

pub fn route() -> Router<()> {
    let state = SharedState::default();

    Router::new()
        .route("/info", get(info::handler))
        .route("/fp-setup", post(fp_setup::handler))
        // Custom RTSP methods
        .route(
            "/:media_id",
            any(|req: Request| async move {
                // TODO : impl empty handlers
                match req.method().as_str() {
                    "SETUP" => setup::handler.call(req, state).await,
                    "GET_PARAMETER" => get_parameter::handler.call(req, state).await,
                    method => {
                        tracing::error!(?method, "unknown method");
                        generic::trace_body.call(req, ()).await
                    }
                }
            }),
        )
        .fallback(generic::trace_body)
        // Unknown handlers, just trace response
        // CSeq is required for RTSP protocol
        .layer(PropagateHeaderLayer::new(HeaderName::from_static("cseq")))
        // Synthetic header to let mapper know that's RTSP, not HTTP
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("upgrade"),
            |_: &_| Some(HeaderValue::from_static("RTSP")),
        ))
}

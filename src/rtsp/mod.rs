use axum::{
    extract::Request,
    handler::Handler,
    http::{HeaderName, HeaderValue},
    response::IntoResponse,
    routing::{any, get, options, post},
    Router,
};
use hyper::StatusCode;
use tower_http::{propagate_header::PropagateHeaderLayer, set_header::SetResponseHeaderLayer};

use self::state::Connections;

mod fp_setup;
mod get_parameter;
mod info;
mod record;
mod setpeers;
mod setup;
mod teardown;

mod plist;
mod state;

pub fn route() -> Router<()> {
    let connections = Connections::default();

    Router::new()
        .route("/", options(()))
        .route("/info", get(info::handler))
        .route("/fp-setup", post(fp_setup::handler))
        .route(
            "/:media_id",
            any(|req: Request| async move {
                match req.method().as_str() {
                    "SETUP" => setup::handler.call(req, connections).await,
                    "RECORD" => record::handler.call(req, connections).await,
                    "SETPEERS" => setpeers::handler.call(req, connections).await,
                    "TEARDOWN" => teardown::handler.call(req, connections).await,
                    "GET_PARAMETER" => get_parameter::handler.call(req, connections).await,
                    other => (StatusCode::BAD_GATEWAY, format!("Unknown method: {other}"))
                        .into_response(),
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

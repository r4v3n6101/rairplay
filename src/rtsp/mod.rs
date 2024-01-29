use axum::{http::HeaderName, routing::get, Router};
use tower::ServiceBuilder;
use tower_http::ServiceBuilderExt;

mod info;
mod plist;

const RTSP_UPGRADE: [(&str, &str); 1] = [("Upgrade", "RTSP")];

pub struct AirplayServer;

impl AirplayServer {
    pub fn new() -> Router<()> {
        let layer = ServiceBuilder::new()
            .propagate_header(HeaderName::from_static("cseq"))
            .trace_for_http();

        Router::<()>::new()
            .route("/rtsp/info", get(info::handler))
            .layer(layer)
    }
}

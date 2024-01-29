use axum::{
    http::{HeaderName, HeaderValue},
    routing::get,
    Router,
};
use tower::ServiceBuilder;
use tower_http::ServiceBuilderExt;

mod info;
mod plist;

pub struct AirplayServer;

impl AirplayServer {
    pub fn new() -> Router<()> {
        let layer = ServiceBuilder::new()
            .override_response_header(HeaderName::from_static("upgrade"), |_: &_| {
                Some(HeaderValue::from_static("RTSP"))
            })
            .propagate_header(HeaderName::from_static("cseq"))
            .trace_for_http();

        Router::<()>::new()
            .route("/rtsp/info", get(info::handler))
            .layer(layer)
    }
}

use std::sync::Arc;

use axum::{
    extract::Request,
    handler::Handler,
    http::{HeaderName, HeaderValue},
    response::IntoResponse,
    routing::{any, get, post},
    Router,
};
use hyper::StatusCode;
use tower::ServiceBuilder;
use tower_http::ServiceBuilderExt;

use crate::info::AppInfo;

mod fp_setup;
mod info;
mod plist;
mod record;
mod setpeers;
mod setup;
mod state;
mod teardown;

pub fn rtsp_service(app_info: AppInfo, initial_volume: f32) -> Router<()> {
    let layer = ServiceBuilder::new()
        .override_response_header(HeaderName::from_static("upgrade"), |_: &_| {
            Some(HeaderValue::from_static("RTSP"))
        })
        .propagate_header(HeaderName::from_static("cseq"))
        .trace_for_http();

    Router::new()
        .route(
            "/info",
            get(info::handler).with_state(info::ServiceInfo {
                app_info: Arc::new(app_info),
                initial_volume,
            }),
        )
        .route("/fp-setup", post(fp_setup::handler))
        .route(
            "/:media_id",
            any(|req: Request| async move {
                match req.method().as_str() {
                    "SETUP" => setup::handler.call(req, ()).await,
                    "RECORD" => record::handler.call(req, ()).await,
                    "SETPEERS" => setpeers::handler.call(req, ()).await,
                    "TEARDOWN" => teardown::handler.call(req, ()).await,
                    other => (StatusCode::BAD_GATEWAY, format!("Unknown method: {other}"))
                        .into_response(),
                }
            }),
        )
        .layer(layer)
}

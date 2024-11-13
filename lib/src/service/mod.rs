use std::{
    future::{Future, IntoFuture},
    pin::Pin,
};

use axum::{
    extract::Request,
    handler::Handler,
    http::{HeaderName, HeaderValue},
    routing::{any, get, post},
    Router,
};
use tokio::net::TcpListener;
use tower_http::{propagate_header::PropagateHeaderLayer, set_header::SetResponseHeaderLayer};

mod dto;
mod handlers;
mod plist;
mod streaming;
mod transport;

// TODO : rename
pub struct IdkRouter {
    svc_listener: TcpListener,
}

impl IdkRouter {
    pub fn new(svc_listener: TcpListener) -> Self {
        Self { svc_listener }
    }
}

impl IntoFuture for IdkRouter {
    type Output = ();

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(transport::serve_with_rtsp_remap(
            self.svc_listener,
            // TODO : move out and remove /rtsp nesting here
            Router::new().nest(
                "/rtsp",
                Router::new()
                    // Heartbeat
                    .route("/feedback", post(()))
                    // I guess it will never be used
                    .route("/command", post(()))
                    // General info about server
                    .route("/info", get(handlers::info))
                    // Fair play, but seems like it isn't working correct
                    .route("/fp-setup", post(handlers::fp_setup))
                    // Custom RTSP methods
                    .route(
                        "/:media_id",
                        any(|req: Request| async move {
                            // TODO : impl empty handlers
                            match req.method().as_str() {
                                "SETUP" => handlers::setup.call(req, ()).await,
                                // Get parameters such as volume
                                "GET_PARAMETER" => handlers::get_parameter.call(req, ()).await,
                                // Set parameters such as artwork, volume, position
                                //"SET_PARAMETER" => set_parameter::handler.call(req, state).await,
                                "SETRATEANCHORTIME" => {
                                    handlers::set_rate_anchor_time.call(req, ()).await
                                }
                                // Remove stream handle and closes all its channels
                                // "TEARDOWN" => handlers::teardown.call(req, ()).await,
                                // Flush remained data, called before teardown
                                "FLUSHBUFFERED" => handlers::flush_buffered.call(req, ()).await,
                                method => {
                                    tracing::error!(?method, path = ?req.uri(), "unknown method");
                                    handlers::generic.call(req, ()).await
                                }
                            }
                        }),
                    )
                    // Unknown handlers, just trace response
                    .fallback(handlers::generic)
                    // CSeq is required for RTSP protocol
                    .layer(PropagateHeaderLayer::new(HeaderName::from_static("cseq")))
                    // Synthetic header to let mapper know that's RTSP, not HTTP
                    .layer(SetResponseHeaderLayer::overriding(
                        HeaderName::from_static("upgrade"),
                        |_: &_| Some(HeaderValue::from_static("RTSP")),
                    )))
                    .into_make_service_with_connect_info::<transport::IncomingStream>(),
        ))
    }
}

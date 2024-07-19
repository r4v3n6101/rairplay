use axum::{
    extract::Request,
    handler::Handler,
    http::{HeaderName, HeaderValue},
    routing::{any, get, post},
    Router,
};
use tokio::net::TcpListener;
use tower_http::{
    propagate_header::PropagateHeaderLayer, set_header::SetResponseHeaderLayer, trace::TraceLayer,
};

mod dto;
mod fairplay;
mod handlers;
mod plist;
mod state;
mod transport;

fn route() -> Router<()> {
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
                    "SETRATEANCHORTIME" => handlers::set_rate_anchor_time.call(req, ()).await,
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
        ))
}

pub async fn start_rtsp_service() {
    let tcp_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();

    let router = Router::new()
        .nest("/rtsp", route())
        .layer(TraceLayer::new_for_http());
    transport::serve_with_rtsp_remap(
        tcp_listener,
        router.into_make_service_with_connect_info::<transport::IncomingStream>(),
    )
    .await;
}

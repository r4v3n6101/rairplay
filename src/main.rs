use axum::Router;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::Level;
use transport::{serve_with_rtsp_remap, IncomingStream};

mod channels;
mod clock;
mod service;

// TODO : re-organize
mod adv;
mod feats;
mod transport;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .pretty()
        .init();

    let tcp_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();

    let router = Router::new()
        .nest("/rtsp", service::route())
        .layer(TraceLayer::new_for_http());
    serve_with_rtsp_remap(
        tcp_listener,
        router.into_make_service_with_connect_info::<IncomingStream>(),
    )
    .await;
}

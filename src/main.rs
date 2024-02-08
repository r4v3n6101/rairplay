use std::sync::Arc;

use advertisment::AdvData;
use axum::Router;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::Level;
use transport::{serve_with_rtsp_remap, IncomingStream};

mod advertisment;
mod rtsp;
mod transport;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let tcp_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();

    let adv_data = Arc::new(AdvData::default());
    let router = Router::new()
        .nest("/rtsp", rtsp::route())
        .layer(TraceLayer::new_for_http());
    serve_with_rtsp_remap(
        tcp_listener,
        adv_data,
        router.into_make_service_with_connect_info::<IncomingStream>(),
    )
    .await;
}

use std::sync::Arc;

use adv::Advertisment;
use axum::Router;
use feats::Features;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::Level;
use transport::{serve_with_rtsp_remap, IncomingStream};

mod channels;
mod clock;
mod service;

// TODO : move out modules from old layout
mod adv;
mod feats;
mod plist;
mod rtsp;
mod transport;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .pretty()
        .init();

    let tcp_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();

    let features = Features::default()
        | Features::ScreenMirroring
        | Features::ScreenRotate
        | Features::Video
        | Features::Photo
        | Features::VideoHTTPLiveStreaming;
    let mut adv = Advertisment::default();
    adv.features = features;
    let adv = Arc::new(adv);
    adv.features.validate();

    let router = Router::new()
        .nest("/rtsp", rtsp::route(Arc::clone(&adv)))
        .layer(TraceLayer::new_for_http());
    serve_with_rtsp_remap(
        tcp_listener,
        router.into_make_service_with_connect_info::<IncomingStream>(),
    )
    .await;
}

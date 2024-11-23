use std::net::SocketAddr;

use rairplay::{info::Config, rtsp};
use tokio::net::TcpListener;
use tracing::Level;

mod discovery;
mod transport;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .pretty()
        .init();

    let cfg = Config {
        mac_addr: [0x9f, 0xd7, 0xaf, 0x1f, 0xd3, 0xcd].into(),
        features: Default::default(),
        initial_volume: Default::default(),

        manufacturer: env!("CARGO_PKG_AUTHORS").to_string(),
        model: env!("CARGO_PKG_NAME").to_string(),
        name: env!("CARGO_PKG_NAME").to_string(),
        fw_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let svc_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();
    discovery::mdns_broadcast();
    transport::serve_with_rtsp_remap(
        svc_listener,
        rtsp::svc_router(cfg.clone()).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await;
}

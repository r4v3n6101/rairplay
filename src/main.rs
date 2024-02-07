use axum::Router;
use info::AppInfo;
use tokio::net::TcpListener;
use tracing::Level;

mod info;
mod rtsp;
mod transport;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .pretty()
        .with_max_level(Level::DEBUG)
        .init();

    let tcp_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();

    let app_info = AppInfo {
        mac_addr: [0x10, 0x12, 0x32, 0x64, 0x21, 0xFF],
        manufacturer: env!("CARGO_PKG_AUTHORS").to_string(),
        model: env!("CARGO_PKG_NAME").to_string(),
        name: env!("CARGO_PKG_NAME").to_string(),
        fw_version: env!("CARGO_PKG_VERSION").to_string(),
        pin: None,
    };
    let rtsp_svc = rtsp::rtsp_service(app_info, -144.0);
    let router = Router::new().nest("/rtsp", rtsp_svc);

    transport::serve_with_rtsp_remap(tcp_listener, router).await;
}

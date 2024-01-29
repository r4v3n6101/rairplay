use tokio::net::TcpListener;
use tracing::Level;

mod rtsp;
mod transport;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .pretty()
        .with_max_level(Level::DEBUG)
        .init();

    let tcp_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();
    let svc = rtsp::AirplayServer::new();

    transport::serve_with_rtsp_remap(tcp_listener, svc).await;
}

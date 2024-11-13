use tokio::net::TcpListener;
use tracing::Level;

mod discovery;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .pretty()
        .init();

    let svc_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();
    discovery::mdns_broadcast();
    rairplay::service::IdkRouter::new(svc_listener).await;
}

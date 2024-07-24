use tracing::Level;

mod clock;
mod discovery;
mod service;
mod streaming;

// TODO : re-organize
mod adv;
mod feats;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .pretty()
        .init();

    discovery::mdns_broadcast();
    service::start_rtsp_service().await;
}

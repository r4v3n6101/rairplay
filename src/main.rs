use tracing::Level;

mod channels;
mod clock;
mod discovery;
mod service;

// TODO : re-organize
mod adv;
mod feats;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .pretty()
        .init();

    discovery::mdns_broadcast();
    service::start_rtsp_service().await;
}

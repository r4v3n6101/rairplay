use tracing::Level;

mod channels;
mod clock;
mod service;
mod discovery;

// TODO : re-organize
mod adv;
mod feats;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .pretty()
        .init();

    service::start_rtsp_service().await;
}

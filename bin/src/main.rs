use rairplay::playback::null::NullDevice;
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

    let svc_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();
    discovery::mdns_broadcast();
    transport::serve_with_rtsp_remap(
        svc_listener,
        rairplay::rtsp::RouterService::serve(rairplay::config::Config::<
            NullDevice<_, _>,
            NullDevice<_, _>,
        >::default()),
    )
    .await;
}

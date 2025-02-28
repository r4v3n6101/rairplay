use std::net::SocketAddr;

use rairplay::{svc_router, Config};
use tokio::net::TcpListener;
use tower::{service_fn, Service};
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
        service_fn(move |addr| {
            svc_router(Config::default())
                .into_make_service_with_connect_info::<SocketAddr>()
                .call(addr)
        }),
    )
    .await;
}

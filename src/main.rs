use mdns_sd::{ServiceDaemon, ServiceInfo};
use publish::Airplay2TXTRecords;
use tokio::net::TcpListener;

mod publish;
mod server;
mod transport;

#[tokio::main]
async fn main() {
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    // Create a service info.
    let props = Airplay2TXTRecords::default();

    let (ip, port) = ("192.168.1.67", 5200);
    let my_service = ServiceInfo::new(
        "_airplay._tcp.local.",
        "whateva",
        "192.168.1.67.local.",
        ip,
        port,
        &[
            ("deviceid", props.device_id.as_str()),
            ("features", props.features.as_str()),
            ("model", props.model.as_str()),
        ][..],
    )
    .unwrap();

    // Register with the daemon, which publishes the service.
    mdns.register(my_service)
        .expect("Failed to register our service");

    let tcp_listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    let svc = server::AirplayServer::new();

    transport::serve_with_rtsp_remap(tcp_listener, svc).await;

    // Gracefully shutdown the daemon
    std::thread::sleep(std::time::Duration::from_secs(30));
    mdns.shutdown().unwrap();
}

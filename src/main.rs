use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6},
    sync::Arc,
};

use rairplay::{
    config::{Audio, Config, DefaultKeychain, Features, Pairing, Video},
    transport::DualStackListenerWithRtspRemap,
};
use tracing::level_filters::LevelFilter;

mod audio;
mod discovery;
mod playback;
mod video;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .pretty()
        .init();

    gstreamer::init().expect("gstreamer initialization");

    let mut features = Features::default();
    features.insert(Features::HomeKitPairing);

    let config = Arc::new(Config::<_, _, DefaultKeychain> {
        name: "rairplay".to_string(),
        video: Video {
            device: playback::PipeDevice {
                callback: video::transcode,
            },
            ..Default::default()
        },
        audio: Audio {
            device: playback::PipeDevice {
                callback: audio::transcode,
            },
            ..Default::default()
        },
        pairing: Pairing::HomeKit,
        features,
        ..Default::default()
    });

    discovery::mdns_broadcast(config.as_ref());

    let ap_svc = rairplay::ServiceFactory::new(config);
    axum::serve(
        DualStackListenerWithRtspRemap::bind(
            SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 5200),
            SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 5200, 0, 0),
        )
        .unwrap(),
        ap_svc,
    )
    .await
    .unwrap();
}

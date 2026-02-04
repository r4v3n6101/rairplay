use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6},
    sync::Arc,
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

    let config = Arc::new(
        airplay::config::Config::<_, _, airplay::config::DefaultKeychain> {
            name: "rairplay".to_string(),
            video: airplay::config::Video {
                device: playback::PipeDevice {
                    callback: video::transcode,
                },
                ..Default::default()
            },
            audio: airplay::config::Audio {
                device: playback::PipeDevice {
                    callback: audio::transcode,
                },
                ..Default::default()
            },
            pairing: airplay::config::Pairing::HomeKit,
            ..Default::default()
        },
    );

    discovery::mdns_broadcast(config.as_ref());

    axum::serve(
        airplay::rtsp::Listener::bind(
            SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 5200),
            SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 5200, 0, 0),
        )
        .await
        .unwrap(),
        airplay::rtsp::service_factory(config),
    )
    .await
    .unwrap();
}

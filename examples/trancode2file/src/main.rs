use std::net::Ipv6Addr;
use std::sync::Arc;

use airplay::net::{bind_addr, bind_tcp_dual_stack};
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

    let config = Arc::new(airplay::config::Config::<_, _> {
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
        ..Default::default()
    });

    discovery::mdns_broadcast(config.as_ref());

    let tcp_listener = bind_tcp_dual_stack(bind_addr(
        Ipv6Addr::UNSPECIFIED.into(),
        5200,
    ))
    .await
    .unwrap();
    axum::serve(
        airplay::rtsp::Listener { tcp_listener },
        airplay::rtsp::service_factory(config),
    )
    .await
    .unwrap();
}

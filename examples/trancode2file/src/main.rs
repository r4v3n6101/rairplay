use std::sync::Arc;

use tokio::net::TcpListener;
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

    discovery::mdns_broadcast();

    let config = Arc::new(airplay::config::Config::<_, _> {
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

    let tcp_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();
    axum::serve(
        airplay::rtsp::Listener { tcp_listener },
        airplay::rtsp::service_factory(config),
    )
    .await
    .unwrap();
}

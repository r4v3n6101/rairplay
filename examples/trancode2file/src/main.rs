use std::sync::Arc;

use tokio::net::TcpListener;
use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod audio;
mod discovery;
mod playback;
mod transport;
mod video;

#[tokio::main]
async fn main() {
    let (chrome_layer, _guard) = ChromeLayerBuilder::new().build();
    tracing_subscriber::registry().with(chrome_layer).init();

    gstreamer::init().expect("gstreamer initialization");

    let svc_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();
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

    transport::serve_with_rtsp_remap(svc_listener, airplay::rtsp::RtspService { config }).await;
}

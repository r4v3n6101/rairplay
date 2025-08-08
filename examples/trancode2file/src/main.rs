use tokio::net::TcpListener;
use tracing::Level;

mod audio;
mod discovery;
mod playback;
mod transport;
mod video;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .pretty()
        .init();

    gstreamer::init().expect("gstreamer initialization");

    let svc_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();
    discovery::mdns_broadcast();

    let cfg = rairplay::config::Config::<_, _> {
        video: rairplay::config::Video {
            device: playback::PipeDevice {
                callback: video::transcode,
            },
            ..Default::default()
        },
        audio: rairplay::config::Audio {
            device: playback::PipeDevice {
                callback: audio::transcode,
            },
            ..Default::default()
        },
        ..Default::default()
    };

    transport::serve_with_rtsp_remap(svc_listener, rairplay::rtsp::RouterService::serve(cfg)).await;
}

use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6},
    sync::Arc,
    thread,
};

use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use example_common::{audio, discovery, playback};
mod video;
mod ui;

fn main() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy()
        .add_directive("mdns_sd=off".parse().expect("mdns filter"));

    tracing_subscriber::fmt().with_env_filter(filter).pretty().init();

    gstreamer::init().expect("gstreamer initialization");

    let shared_frame = video::init_shared_frame();
    let (ui_tx, ui_rx) = tokio::sync::mpsc::unbounded_channel();
    video::set_ui_sender(ui_tx);

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

    let (restart_tx, restart_rx) = std::sync::mpsc::channel::<()>();
    let server_thread = thread::spawn(move || {
        loop {
            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
            let config = Arc::clone(&config);

            let handle = thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
                runtime.block_on(async move {
                    let shutdown = async move {
                        let _ = shutdown_rx.await;
                    };
                    axum::serve(
                        airplay::rtsp::Listener::bind(
                            SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 5200),
                            SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 5200, 0, 0),
                        )
                        .await
                        .unwrap(),
                        airplay::rtsp::service_factory(config),
                    )
                    .with_graceful_shutdown(shutdown)
                    .await
                    .unwrap();
                });
            });

            if restart_rx.recv().is_err() {
                let _ = shutdown_tx.send(());
                let _ = handle.join();
                break;
            }

            let _ = shutdown_tx.send(());
            let _ = handle.join();
        }
    });

    ui::run_video_window(shared_frame, ui_rx, restart_tx);
    let _ = server_thread.join();
}

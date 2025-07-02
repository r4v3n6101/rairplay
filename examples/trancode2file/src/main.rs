use std::{
    convert::Infallible,
    error::Error,
    sync::{mpsc, Weak},
    thread,
};

use rairplay::playback::{
    null::NullDevice,
    video::{VideoDevice, VideoPacket, VideoParams},
    ChannelHandle, Device, Stream,
};
use tokio::net::TcpListener;
use tracing::Level;

mod discovery;
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

    let cfg = rairplay::config::Config::<NullDevice<_, _>, PipeDevice> {
        video: rairplay::config::Video {
            device: PipeDevice,
            ..Default::default()
        },
        ..Default::default()
    };

    transport::serve_with_rtsp_remap(svc_listener, rairplay::rtsp::RouterService::serve(cfg)).await;
}

#[derive(Debug, Default)]
pub struct PipeDevice;

impl Device for PipeDevice {
    type Params = VideoParams;
    type Stream = PipeStream<VideoPacket>;
    type Error = Infallible;

    fn create(
        &self,
        id: u64,
        _: Self::Params,
        handle: Weak<dyn ChannelHandle>,
    ) -> Result<Self::Stream, Self::Error> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            if let Err(err) = video::transcode(rx, id) {
                tracing::error!(%err, "error during transcoding video");
            }
            if let Some(handle) = handle.upgrade() {
                handle.close();
            }
        });

        Ok(PipeStream {
            id: format!("transcode_video_stream_{id}"),
            tx,
        })
    }
}

impl VideoDevice for PipeDevice {}

/// Stream that just pipes packets through channel
pub struct PipeStream<T> {
    id: String,
    tx: mpsc::Sender<T>,
}

impl<T> Stream for PipeStream<T>
where
    T: 'static + Send,
{
    type Content = T;

    fn on_data(&self, content: Self::Content) {
        let _ = self.tx.send(content);
    }

    fn on_ok(self) {
        tracing::info!(id=%self.id, "pipe stream successfully closed");
    }

    fn on_err(self, err: Box<dyn Error>) {
        tracing::error!(%err, id=%self.id, "pipe stream ended with an error");
    }
}

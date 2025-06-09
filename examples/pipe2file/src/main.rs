use std::{
    convert::Infallible,
    error::Error,
    fs::File,
    path::PathBuf,
    sync::{mpsc, Weak},
    thread,
};

use clap::Parser;
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    video_output: PathBuf,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .pretty()
        .init();

    ffmpeg_next::init().unwrap();

    let args = Args::parse();

    let svc_listener = TcpListener::bind("0.0.0.0:5200").await.unwrap();
    discovery::mdns_broadcast();

    let cfg = rairplay::config::Config::<NullDevice<_, _>, PipeDevice> {
        video: rairplay::config::Video {
            device: PipeDevice {
                output: args.video_output,
            },
            ..Default::default()
        },
        ..Default::default()
    };

    transport::serve_with_rtsp_remap(svc_listener, rairplay::rtsp::RouterService::serve(cfg)).await;
}

#[derive(Debug, Default)]
pub struct PipeDevice {
    output: PathBuf,
}

impl Device for PipeDevice {
    type Params = VideoParams;
    type Stream = PipeStream<VideoPacket>;
    type Error = Infallible;

    fn create(
        &self,
        _: Self::Params,
        handle: Weak<dyn ChannelHandle>,
    ) -> Result<Self::Stream, Self::Error> {
        let (tx, rx) = mpsc::channel();
        let path_buf = self.output.clone();
        thread::spawn(move || {
            if let Err(err) = video::transcode(rx, File::create(path_buf).expect("opened file")) {
                tracing::error!(%err, "ffmpeg error during transcoding video");
            }
            if let Some(handle) = handle.upgrade() {
                handle.close();
            }
        });

        Ok(PipeStream {
            id: "ffmpeg_file_pipe",
            tx,
        })
    }
}

impl VideoDevice for PipeDevice {}

/// Stream that just pipes packets through channel
pub struct PipeStream<T> {
    id: &'static str,
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
        tracing::info!(id=%self.id, "stream successfully closed");
    }

    fn on_err(self, err: Box<dyn Error>) {
        tracing::error!(%err, id=%self.id, "stream closed with an error");
    }
}

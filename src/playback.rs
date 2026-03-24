use std::{
    convert::Infallible,
    error::Error,
    future::Future,
    sync::{Weak, mpsc},
    thread,
};

use airplay::playback::{
    ChannelHandle, Device, Stream,
    audio::{AudioDevice, AudioPacket, AudioParams},
    video::{VideoDevice, VideoPacket, VideoParams},
};

pub type PipeCallback<Params, Packet> =
    fn(u64, Params, mpsc::Receiver<Packet>) -> Result<(), Box<dyn Error>>;

#[derive(Debug)]
pub struct PipeDevice<Params, Packet> {
    pub callback: PipeCallback<Params, Packet>,
}

impl<Params, Packet> Default for PipeDevice<Params, Packet> {
    fn default() -> Self {
        Self {
            callback: noop::<Params, Packet>,
        }
    }
}

impl<Params, Packet> Device for PipeDevice<Params, Packet>
where
    Params: Send + Sync + 'static,
    Packet: Send + Sync + 'static,
{
    type Params = Params;
    type Stream = PipeStream<Packet>;
    type Error = Infallible;

    fn create(
        &self,
        id: u64,
        params: Self::Params,
        handle: Weak<dyn ChannelHandle>,
    ) -> impl Future<Output = Result<Self::Stream, Self::Error>> + Send {
        let (tx, rx) = mpsc::channel();
        let callback = self.callback;
        thread::spawn(move || {
            if let Err(err) = (callback)(id, params, rx) {
                tracing::error!(%err, %id, "error during transcoding");
            }
            if let Some(handle) = handle.upgrade() {
                handle.close();
            }
        });

        async move {
            Ok(PipeStream {
                id: format!("stream_{id}"),
                tx,
            })
        }
    }
}

impl VideoDevice for PipeDevice<VideoParams, VideoPacket> {}

impl AudioDevice for PipeDevice<AudioParams, AudioPacket> {
    fn get_volume(&self) -> f32 {
        0.0
    }

    fn set_volume(&self, _: f32) {}
}

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

#[inline]
fn noop<Params, Packet>(
    _: u64,
    _: Params,
    _: mpsc::Receiver<Packet>,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

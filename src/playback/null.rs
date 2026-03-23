use std::{
    convert::Infallible, error::Error, fmt, future::Future, marker::PhantomData, sync::Weak,
};

use super::{
    ChannelHandle, Device, Stream,
    audio::{AudioDevice, AudioPacket, AudioParams},
    video::{VideoDevice, VideoPacket, VideoParams},
};

pub struct NullDevice<Params, Content>(PhantomData<(Params, Content)>);

unsafe impl<P, C> Send for NullDevice<P, C> {}
unsafe impl<P, C> Sync for NullDevice<P, C> {}

impl<P, C> Default for NullDevice<P, C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<P, C> Device for NullDevice<P, C>
where
    P: fmt::Debug + 'static,
    C: fmt::Debug + 'static,
{
    type Params = P;
    type Stream = NullStream<C>;
    type Error = Infallible;

    fn create(
        &self,
        id: u64,
        params: Self::Params,
        _: Weak<dyn ChannelHandle>,
    ) -> impl Future<Output = Result<Self::Stream, Self::Error>> + Send {
        tracing::info!(?params, %id, "created null stream");
        async { Ok(NullStream(PhantomData)) }
    }
}

impl AudioDevice for NullDevice<AudioParams, AudioPacket> {
    fn get_volume(&self) -> f32 {
        tracing::debug!("volume requested for null stream");
        0.0
    }

    fn set_volume(&self, value: f32) {
        tracing::debug!(%value, "volume changed for null stream");
    }
}

impl VideoDevice for NullDevice<VideoParams, VideoPacket> {}

pub struct NullStream<C>(PhantomData<C>);

unsafe impl<C> Send for NullStream<C> {}
unsafe impl<C> Sync for NullStream<C> {}

impl<C> Stream for NullStream<C>
where
    C: fmt::Debug + 'static,
{
    type Content = C;

    fn on_data(&self, content: Self::Content) {
        tracing::trace!(?content, "stream feed with content");
    }

    fn on_ok(self) {
        tracing::info!("null stream finished successfully");
    }

    fn on_err(self, err: Box<dyn Error>) {
        tracing::error!(%err, "null stream finished with an error");
    }
}

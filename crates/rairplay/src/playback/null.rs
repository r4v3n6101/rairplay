use std::{convert::Infallible, fmt, future::Future, marker::PhantomData, sync::Weak};

use super::{
    ChannelHandle, Device, EndReason, Stream,
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
    type Error = Infallible;

    fn on_data(&self, content: Self::Content) -> Result<(), Self::Error> {
        tracing::trace!(?content, "stream feed with content");
        Ok(())
    }

    fn on_end(self, reason: EndReason<Self::Error>) {
        match reason {
            EndReason::Finished => tracing::info!("null stream finished successfully"),
            EndReason::Cancelled => tracing::info!("null stream cancelled"),
            EndReason::Transport(err) => {
                tracing::error!(%err, "null stream finished with an error")
            }
            EndReason::Backend(err) => match err {},
        }
    }
}

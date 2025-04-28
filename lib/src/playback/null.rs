use std::{error::Error, fmt, marker::PhantomData, sync::Weak};

use super::{ChannelHandle, Device, Stream};

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

    fn create(&self, params: Self::Params, _: Weak<dyn ChannelHandle>) -> Self::Stream {
        tracing::info!(?params, "created null stream");
        NullStream(PhantomData)
    }
}

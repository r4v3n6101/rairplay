use std::{marker::PhantomData, time::Duration};

pub struct BufferedData<T> {
    pub wait_until_next: Option<Duration>,
    pub data: Vec<T>,
}

pub struct AudioParams {
    pub sample_rate: u32,
}

pub struct VideoParams {
    pub fps: u32,
}

pub trait Device: Send + Sync {
    type Content;
    type Params;

    fn create(
        &self,
        params: Self::Params,
        data_provider: Box<dyn FnMut() -> BufferedData<Self::Content>>,
    ) -> Box<dyn Stream>;
}

pub trait Stream: Send + Sync + 'static {
    fn flush(&self);
}

pub trait AudioStream: Stream {
    fn get_volume(&self) -> f32;
    fn change_volume(&self, value: f32);
}

pub trait VideoStream: Stream {}

/// Default implementation that does nothing
pub struct NullDevice<C, P>(PhantomData<(C, P)>);

unsafe impl<C, P> Send for NullDevice<C, P> {}
unsafe impl<C, P> Sync for NullDevice<C, P> {}

impl<C, P> Default for NullDevice<C, P> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

impl<C, P> Device for NullDevice<C, P> {
    type Content = C;
    type Params = P;

    fn create(
        &self,
        _: Self::Params,
        _: Box<dyn FnMut() -> BufferedData<Self::Content>>,
    ) -> Box<dyn Stream> {
        tracing::debug!("created null stream");
        Box::new(NullStream(()))
    }
}

pub struct NullStream(());

impl Stream for NullStream {
    fn flush(&self) {
        tracing::debug!("flush called for null stream");
    }
}

impl AudioStream for NullStream {
    fn get_volume(&self) -> f32 {
        tracing::debug!("volume requested for null stream");
        0.0
    }

    fn change_volume(&self, value: f32) {
        tracing::debug!(%value, "changed volume for null stream")
    }
}

impl VideoStream for NullStream {}

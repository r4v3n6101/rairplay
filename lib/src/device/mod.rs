use std::{marker::PhantomData, time::Duration};

pub type DataCallback<T> = Box<dyn FnMut() -> BufferedData<T>>;

pub struct BufferedData<T> {
    pub wait_until_next: Option<Duration>,
    pub data: Vec<T>,
}

#[derive(Debug, Clone, Copy)]
pub struct AudioParams {
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct VideoParams {
    pub fps: u32,
}

pub trait Device: Send + Sync {
    type Content;
    type Params;

    fn create(
        &self,
        params: Self::Params,
        data_callback: DataCallback<Self::Content>,
    ) -> Box<dyn Stream>;
}

pub trait Stream: Send + Sync + 'static {
    fn flush(&self);
}

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

    fn create(&self, _: Self::Params, _: DataCallback<Self::Content>) -> Box<dyn Stream> {
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

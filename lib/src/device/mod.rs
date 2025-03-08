use std::{fmt, marker::PhantomData, time::Duration};

pub type DataCallback<T> = Box<dyn FnMut() -> BufferedData<T>>;

pub struct BufferedData<T> {
    pub wait_until_next: Option<Duration>,
    pub data: Vec<T>,
}

#[derive(Debug, Clone, Copy)]
pub struct AudioParams {
    pub samples_per_frame: u32,
    pub codec: Codec,
}

#[derive(Debug, Clone, Copy)]
pub struct Codec {
    pub kind: CodecKind,
    pub bits_per_sample: u32,
    pub sample_rate: u32,
    pub channels: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum CodecKind {
    PCM,
    AAC,
    OPUS,
    ALAC,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct VideoParams {}

pub trait StreamHandle: Send + Sync + 'static {}

pub trait Device: Send + Sync {
    type Content;
    type Params;

    fn create(
        &self,
        params: Self::Params,
        data_callback: DataCallback<Self::Content>,
    ) -> Box<dyn StreamHandle>;
}

pub trait AudioDevice: Device<Content = (), Params = AudioParams> {
    fn get_volume(&self) -> f32;
    fn set_volume(&self, value: f32);
}

pub trait VideoDevice: Device<Content = (), Params = VideoParams> {}

pub struct NullDevice<C, P>(PhantomData<(C, P)>);

unsafe impl<C, P> Send for NullDevice<C, P> {}
unsafe impl<C, P> Sync for NullDevice<C, P> {}

impl<C, P> Default for NullDevice<C, P> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C, P: fmt::Debug> Device for NullDevice<C, P> {
    type Content = C;
    type Params = P;

    fn create(
        &self,
        params: Self::Params,
        _: DataCallback<Self::Content>,
    ) -> Box<dyn StreamHandle> {
        tracing::info!(?params, "created null stream");
        Box::new(NullStream(()))
    }
}

impl AudioDevice for NullDevice<(), AudioParams> {
    fn get_volume(&self) -> f32 {
        tracing::debug!("volume requested for null stream");
        0.0
    }

    fn set_volume(&self, value: f32) {
        tracing::debug!(%value, "volume changed for null stream");
    }
}

impl VideoDevice for NullDevice<(), VideoParams> {}

pub struct NullStream(());

impl StreamHandle for NullStream {}

impl Drop for NullStream {
    fn drop(&mut self) {
        tracing::info!("null stream closed");
    }
}

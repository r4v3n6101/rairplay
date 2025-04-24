use std::{error::Error, sync::Weak};

pub mod null;

pub trait Device: Send + Sync + 'static {
    type Params;
    type Stream: Stream;

    fn create(&self, params: Self::Params, handle: Weak<dyn ChannelHandle>) -> Self::Stream;
}

pub trait ChannelHandle: Send + Sync + 'static {
    fn close(&self);
}

pub trait Stream: Send + Sync + 'static {
    type Content;

    fn on_data(&self, content: Self::Content);
    fn on_ok(self);
    fn on_err(self, err: Box<dyn Error>);
}

pub trait AudioDevice: Device<Params = AudioParams, Stream: AudioStream> {
    fn get_volume(&self) -> f32;
    fn set_volume(&self, value: f32);
}

pub trait VideoDevice: Device<Params = VideoParams, Stream: VideoStream> {}

pub trait AudioStream: Stream<Content = AudioPacket> {}
impl<T> AudioStream for T where T: Stream<Content = AudioPacket> {}

pub trait VideoStream: Stream<Content = VideoPacket> {}
impl<T> VideoStream for T where T: Stream<Content = VideoPacket> {}

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
    Pcm,
    Aac,
    Opus,
    Alac,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct VideoParams {}

#[derive(Debug)]
pub struct AudioPacket;
#[derive(Debug)]
pub struct VideoPacket;

use std::{fmt, marker::PhantomData, time::Duration};

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
    Pcm,
    Aac,
    Opus,
    Alac,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct VideoParams {}

pub struct AudioPacket;
pub struct VideoPacket;

pub trait DataChannel {
    type Content;

    fn pull_data(&self) -> BufferedData<Self::Content>;
}

pub trait Device: Send + Sync {
    type Params;
    type Channel: DataChannel;

    fn create(&self, params: Self::Params, channel: Self::Channel);
}

pub trait AudioDevice: Device<Params = AudioParams>
where
    Self::Channel: DataChannel<Content = AudioPacket>,
{
    fn get_volume(&self) -> f32;
    fn set_volume(&self, value: f32);
}

pub trait VideoDevice: Device<Params = VideoParams>
where
    Self::Channel: DataChannel<Content = VideoPacket>,
{
}

pub struct NullDevice<Params, Content, Channel>(PhantomData<(Params, Content, Channel)>);

unsafe impl<P, Con, Ch> Send for NullDevice<P, Con, Ch> {}
unsafe impl<P, Con, Ch> Sync for NullDevice<P, Con, Ch> {}

impl<P, Con, Ch> Default for NullDevice<P, Con, Ch> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<Params, Channel, Content> Device for NullDevice<Params, Content, Channel>
where
    Params: fmt::Debug,
    Channel: DataChannel<Content = Content>,
{
    type Params = Params;
    type Channel = Channel;

    fn create(&self, params: Self::Params, _: Self::Channel) {
        tracing::info!(?params, "created null stream");
    }
}

impl<Channel> AudioDevice for NullDevice<AudioParams, AudioPacket, Channel>
where
    Channel: DataChannel<Content = AudioPacket>,
{
    fn get_volume(&self) -> f32 {
        tracing::debug!("volume requested for null stream");
        0.0
    }

    fn set_volume(&self, value: f32) {
        tracing::debug!(%value, "volume changed for null stream");
    }
}

impl<Channel> VideoDevice for NullDevice<VideoParams, VideoPacket, Channel> where
    Channel: DataChannel<Content = VideoPacket>
{
}

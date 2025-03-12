use crate::device::{AudioPacket, BufferedData, DataChannel, VideoPacket};

pub(crate) mod audio;
pub(crate) mod event;
pub(crate) mod video;

enum AudioChannelInner {
    Realtime(audio::RealtimeChannel),
    Buffered(audio::BufferedChannel),
}

pub struct AudioChannel {
    inner: AudioChannelInner,
}

pub struct VideoChannel {
    inner: video::Channel,
}

impl DataChannel for AudioChannel {
    type Content = AudioPacket;

    fn pull_data(&self) -> BufferedData<Self::Content> {
        match &self.inner {
            AudioChannelInner::Realtime(chan) => chan.pull_data(),
            AudioChannelInner::Buffered(chan) => chan.pull_data(),
        }
    }
}

impl DataChannel for VideoChannel {
    type Content = VideoPacket;

    fn pull_data(&self) -> BufferedData<Self::Content> {
        self.inner.pull_data()
    }
}

impl From<audio::RealtimeChannel> for AudioChannel {
    fn from(value: audio::RealtimeChannel) -> Self {
        Self {
            inner: AudioChannelInner::Realtime(value),
        }
    }
}

impl From<audio::BufferedChannel> for AudioChannel {
    fn from(value: audio::BufferedChannel) -> Self {
        Self {
            inner: AudioChannelInner::Buffered(value),
        }
    }
}

impl From<video::Channel> for VideoChannel {
    fn from(value: video::Channel) -> Self {
        Self { inner: value }
    }
}

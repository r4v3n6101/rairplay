use bytes::BytesMut;

use super::{null::NullDevice, Device, Stream};

pub trait VideoDevice: Device<Params = VideoParams, Stream: VideoStream> {}

pub trait VideoStream: Stream<Content = VideoPacket> {}
impl<T> VideoStream for T where T: Stream<Content = VideoPacket> {}

impl VideoDevice for NullDevice<VideoParams, VideoPacket> {}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct VideoParams {}

#[derive(Debug)]
pub struct VideoPacket {
    pub kind: PacketKind,
    pub timestamp: u64,
    pub payload: BytesMut,
}

#[derive(Debug, Clone, Copy)]
pub enum PacketKind {
    AvcC,
    Payload,
    Other(u16),
}

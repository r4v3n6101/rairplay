use bytes::BytesMut;

use super::{Device, Stream};

/// Playback backend for video streams.
pub trait VideoDevice: Device<Params = VideoParams, Stream: VideoStream> {}

/// Stream receiving decrypted video packets.
pub trait VideoStream: Stream<Content = VideoPacket> {}
impl<T> VideoStream for T where T: Stream<Content = VideoPacket> {}

/// Parameters provided when a video stream is created.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct VideoParams {}

/// Decrypted video payload delivered to a [`VideoStream`].
#[derive(Debug)]
pub struct VideoPacket {
    /// Packet classification.
    pub kind: PacketKind,
    /// Stream timestamp associated with the packet.
    pub timestamp: u64,
    /// Packet payload bytes.
    pub payload: BytesMut,
}

/// Kind of video payload delivered to the backend.
#[derive(Debug, Clone, Copy)]
pub enum PacketKind {
    /// AVC decoder configuration record.
    AvcC,
    /// HEVC decoder configuration record.
    HvcC,
    /// Regular encoded video payload.
    Payload,
    /// Auxiliary plist payload.
    Plist,
    /// Unknown packet kind.
    Other(u16),
}

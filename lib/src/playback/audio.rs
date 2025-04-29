use std::fmt;

use bytes::BytesMut;

use super::{null::NullDevice, Device, Stream};

pub trait AudioDevice: Device<Params = AudioParams, Stream: AudioStream> {
    fn get_volume(&self) -> f32;
    fn set_volume(&self, value: f32);
}

pub trait AudioStream: Stream<Content = AudioPacket> {}
impl<T> AudioStream for T where T: Stream<Content = AudioPacket> {}

impl AudioDevice for NullDevice<AudioParams, AudioPacket> {
    fn get_volume(&self) -> f32 {
        tracing::debug!("volume requested for null stream");
        0.0
    }

    fn set_volume(&self, value: f32) {
        tracing::debug!(%value, "volume changed for null stream");
    }
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

#[derive(Debug)]
pub struct AudioPacket {
    pub rtp: RtpPacket,
}

#[derive(Debug)]
pub struct RtpPacket {
    pub(crate) inner: BytesMut,
}

impl RtpPacket {
    pub fn header(&self) -> RtpHeader {
        RtpHeader(
            (self.inner[..RtpHeader::SIZE])
                .try_into()
                // TODO
                .expect("rtp packet must be at least 12 bytes"),
        )
    }

    pub fn payload(&self) -> &[u8] {
        &self.inner[RtpHeader::SIZE..]
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.inner[RtpHeader::SIZE..]
    }
}

impl AsRef<[u8]> for RtpPacket {
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl AsMut<[u8]> for RtpPacket {
    fn as_mut(&mut self) -> &mut [u8] {
        self.inner.as_mut()
    }
}

pub struct RtpHeader([u8; Self::SIZE]);

impl fmt::Debug for RtpHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RtpHeader")
            .field("version", &self.version())
            .field("padding", &self.padding())
            .field("extension", &self.extension())
            .field("csrc_count", &self.csrc_count())
            .field("marker", &self.marker())
            .field("payload_type", &self.payload_type())
            .field("seqnum", &self.seqnum())
            .field("timestamp", &self.timestamp())
            .field("ssrc", &self.ssrc())
            .finish()
    }
}

impl RtpHeader {
    pub const SIZE: usize = 12;

    pub fn version(&self) -> u8 {
        (self.0[0] & 0b1100_0000) >> 6
    }

    pub fn padding(&self) -> u8 {
        (self.0[0] & 0b0010_0000) >> 5
    }

    pub fn extension(&self) -> u8 {
        (self.0[0] & 0b0001_0000) >> 4
    }

    pub fn csrc_count(&self) -> u8 {
        self.0[0] & 0b0000_1111
    }

    pub fn marker(&self) -> bool {
        (self.0[1] & 0b1000_0000) >> 7 == 1
    }

    pub fn payload_type(&self) -> u8 {
        self.0[1] & 0b0111_1111
    }

    pub fn seqnum(&self) -> u16 {
        let mut seqnum = [0; 2];
        seqnum.copy_from_slice(&self.0[2..][..2]);
        u16::from_be_bytes(seqnum)
    }

    pub fn timestamp(&self) -> u32 {
        let mut timestamp = [0; 4];
        timestamp.copy_from_slice(&self.0[4..][..4]);
        u32::from_be_bytes(timestamp)
    }

    pub fn ssrc(&self) -> u32 {
        let mut ssrc = [0; 4];
        ssrc.copy_from_slice(&self.0[8..][..4]);
        u32::from_be_bytes(ssrc)
    }
}

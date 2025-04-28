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
    pub const HEADER_LEN: usize = 12;

    /// # Panics
    /// May panic if packet is less than 12 bytes
    pub fn header(&mut self) -> &mut [u8; Self::HEADER_LEN] {
        (&mut self.inner[..Self::HEADER_LEN])
            .try_into()
            .expect("rtp packet must be at least 12 bytes")
    }

    pub fn payload(&mut self) -> &mut [u8] {
        &mut self.inner[Self::HEADER_LEN..]
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

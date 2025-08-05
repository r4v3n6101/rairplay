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
    pub rtp: BytesMut,
}

impl AudioPacket {
    /// Just RTP header
    pub const HEADER_LEN: usize = 12;
    /// Used for decryption of buffered stream
    /// Won't be stored in Self
    pub const TRAILER_LEN: usize = 24;
}

pub static AUDIO_FORMATS: [Codec; 33] = [
    // 0    Dummy
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 0,
        sample_rate: 0,
        channels: 0,
    },
    // 1    Dummy
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 0,
        sample_rate: 0,
        channels: 0,
    },
    // 2	0x4	PCM/8000/16/1
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 8000,
        channels: 1,
    },
    // 3	0x8	PCM/8000/16/2
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 8000,
        channels: 2,
    },
    // 4	0x10	PCM/16000/16/1
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 16000,
        channels: 1,
    },
    // 5	0x20	PCM/16000/16/2
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 16000,
        channels: 2,
    },
    // 6	0x40	PCM/24000/16/1
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 24000,
        channels: 1,
    },
    // 7	0x80	PCM/24000/16/2
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 24000,
        channels: 2,
    },
    // 8	0x100	PCM/32000/16/1
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 32000,
        channels: 1,
    },
    // 9	0x200	PCM/32000/16/2
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 32000,
        channels: 2,
    },
    // 10	0x400	PCM/44100/16/1
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 44100,
        channels: 1,
    },
    // 11	0x800	PCM/44100/16/2
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 44100,
        channels: 2,
    },
    // 12	0x1000	PCM/44100/24/1
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 24,
        sample_rate: 44100,
        channels: 1,
    },
    // 13	0x2000	PCM/44100/24/2
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 24,
        sample_rate: 44100,
        channels: 2,
    },
    // 14	0x4000	PCM/48000/16/1
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 48000,
        channels: 1,
    },
    // 15	0x8000	PCM/48000/16/2
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 16,
        sample_rate: 48000,
        channels: 2,
    },
    // 16	0x10000	PCM/48000/24/1
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 24,
        sample_rate: 48000,
        channels: 1,
    },
    // 17	0x20000	PCM/48000/24/2
    Codec {
        kind: CodecKind::Pcm,
        bits_per_sample: 24,
        sample_rate: 48000,
        channels: 2,
    },
    // 18	0x40000	ALAC/44100/16/2
    Codec {
        kind: CodecKind::Alac,
        bits_per_sample: 16,
        sample_rate: 44100,
        channels: 2,
    },
    // 19	0x80000	ALAC/44100/24/2
    Codec {
        kind: CodecKind::Alac,
        bits_per_sample: 24,
        sample_rate: 44100,
        channels: 2,
    },
    // 20	0x100000	ALAC/48000/16/2
    Codec {
        kind: CodecKind::Alac,
        bits_per_sample: 16,
        sample_rate: 48000,
        channels: 2,
    },
    // 21	0x200000	ALAC/48000/24/2
    Codec {
        kind: CodecKind::Alac,
        bits_per_sample: 24,
        sample_rate: 48000,
        channels: 2,
    },
    // 22	0x400000	AAC-LC/44100/2
    Codec {
        kind: CodecKind::Aac,
        bits_per_sample: 0,
        sample_rate: 44100,
        channels: 2,
    },
    // 23	0x800000	AAC-LC/48000/2
    Codec {
        kind: CodecKind::Aac,
        bits_per_sample: 0,
        sample_rate: 48000,
        channels: 2,
    },
    // 24	0x1000000	AAC-ELD/44100/2
    Codec {
        kind: CodecKind::Aac,
        bits_per_sample: 0,
        sample_rate: 44100,
        channels: 2,
    },
    // 25	0x2000000	AAC-ELD/48000/2
    Codec {
        kind: CodecKind::Aac,
        bits_per_sample: 0,
        sample_rate: 48000,
        channels: 2,
    },
    // 26	0x4000000	AAC-ELD/16000/1
    Codec {
        kind: CodecKind::Aac,
        bits_per_sample: 0,
        sample_rate: 16000,
        channels: 1,
    },
    // 27	0x8000000	AAC-ELD/24000/1
    Codec {
        kind: CodecKind::Aac,
        bits_per_sample: 0,
        sample_rate: 24000,
        channels: 1,
    },
    // 28	0x10000000	OPUS/16000/1
    Codec {
        kind: CodecKind::Opus,
        bits_per_sample: 0,
        sample_rate: 16000,
        channels: 1,
    },
    // 29	0x20000000	OPUS/24000/1
    Codec {
        kind: CodecKind::Opus,
        bits_per_sample: 0,
        sample_rate: 24000,
        channels: 1,
    },
    // 30	0x40000000	OPUS/48000/1
    Codec {
        kind: CodecKind::Opus,
        bits_per_sample: 0,
        sample_rate: 48000,
        channels: 1,
    },
    // 31	0x80000000	AAC-ELD/44100/1
    Codec {
        kind: CodecKind::Aac,
        bits_per_sample: 0,
        sample_rate: 44100,
        channels: 1,
    },
    // 32	0x100000000	AAC-ELD/48000/1
    Codec {
        kind: CodecKind::Aac,
        bits_per_sample: 0,
        sample_rate: 48000,
        channels: 1,
    },
];

/*
* index       bitflag         format
* 2           0x4             PCM/8000/16/1
* 3           0x8             PCM/8000/16/2
* 4           0x10            PCM/16000/16/1
* 5           0x20            PCM/16000/16/2
* 6           0x40            PCM/24000/16/1
* 7           0x80            PCM/24000/16/2
* 8           0x100           PCM/32000/16/1
* 9           0x200           PCM/32000/16/2
* 10          0x400           PCM/44100/16/1
* 11          0x800           PCM/44100/16/2
* 12          0x1000          PCM/44100/24/1
* 13          0x2000          PCM/44100/24/2
* 14          0x4000          PCM/48000/16/1
* 15          0x8000          PCM/48000/16/2
* 16          0x10000         PCM/48000/24/1
* 17          0x20000         PCM/48000/24/2
* 18          0x40000         ALAC/44100/16/2
* 19          0x80000         ALAC/44100/24/2
* 20          0x100000        ALAC/48000/16/2
* 21          0x200000        ALAC/48000/24/2
* 22          0x400000        AAC-LC/44100/2
* 23          0x800000        AAC-LC/48000/2
* 24          0x1000000       AAC-ELD/44100/2
* 25          0x2000000       AAC-ELD/48000/2
* 26          0x4000000       AAC-ELD/16000/1
* 27          0x8000000       AAC-ELD/24000/1
* 28          0x10000000      OPUS/16000/1
* 29          0x20000000      OPUS/24000/1
* 30          0x40000000      OPUS/48000/1
* 31          0x80000000      AAC-ELD/44100/1
* 32          0x100000000     AAC-ELD/48000/1
*/

/*
const FORMATS: &'static [Format] = &[
    // PCM
    Format::new(2, 0x4, CODEC_TYPE_PCM_S16LE, 8000, 16, 1),
    Format::new(3, 0x8, CODEC_TYPE_PCM_S16LE, 8000, 16, 2),
    Format::new(4, 0x10, CODEC_TYPE_PCM_S16LE, 16000, 16, 1),
    Format::new(5, 0x20, CODEC_TYPE_PCM_S16LE, 16000, 16, 2),
    Format::new(6, 0x40, CODEC_TYPE_PCM_S16LE, 24000, 16, 1),
    Format::new(7, 0x80, CODEC_TYPE_PCM_S16LE, 24000, 16, 2),
    Format::new(8, 0x100, CODEC_TYPE_PCM_S16LE, 32000, 16, 1),
    Format::new(9, 0x200, CODEC_TYPE_PCM_S16LE, 32000, 16, 2),
    Format::new(10, 0x400, CODEC_TYPE_PCM_S16LE, 44100, 16, 1),
    Format::new(11, 0x800, CODEC_TYPE_PCM_S16LE, 44100, 16, 2),
    Format::new(12, 0x1000, CODEC_TYPE_PCM_S24LE, 44100, 24, 1),
    Format::new(13, 0x2000, CODEC_TYPE_PCM_S24LE, 44100, 24, 2),
    Format::new(14, 0x4000, CODEC_TYPE_PCM_S16LE, 48000, 16, 1),
    Format::new(15, 0x8000, CODEC_TYPE_PCM_S16LE, 48000, 16, 2),
    Format::new(16, 0x10000, CODEC_TYPE_PCM_S24LE, 48000, 24, 1),
    Format::new(17, 0x20000, CODEC_TYPE_PCM_S24LE, 48000, 24, 2),
    // ALAC
    Format::new(18, 0x40000, CODEC_TYPE_ALAC, 44100, 16, 2),
    Format::new(19, 0x80000, CODEC_TYPE_ALAC, 44100, 24, 2),
    Format::new(20, 0x100000, CODEC_TYPE_ALAC, 48000, 16, 2),
    Format::new(21, 0x200000, CODEC_TYPE_ALAC, 48000, 24, 2),
    // LD
    Format::new(22, 0x400000, CODEC_TYPE_AAC, 44100, 32, 2),
    Format::new(23, 0x800000, CODEC_TYPE_AAC, 48000, 32, 2),
    // ELD
    Format::new(24, 0x1000000, CODEC_TYPE_AAC, 44100, 32, 2),
    Format::new(25, 0x2000000, CODEC_TYPE_AAC, 48000, 32, 2),
    Format::new(26, 0x4000000, CODEC_TYPE_AAC, 16000, 32, 1),
    Format::new(27, 0x8000000, CODEC_TYPE_AAC, 24000, 32, 1),
    // OPUS
    // TODO : how much bit for decoded samples?
    Format::new(28, 0x10000000, CODEC_TYPE_OPUS, 16000, 0, 1),
    Format::new(29, 0x20000000, CODEC_TYPE_OPUS, 24000, 0, 1),
    Format::new(30, 0x40000000, CODEC_TYPE_OPUS, 48000, 0, 1),
    // ELD
    Format::new(31, 0x80000000, CODEC_TYPE_AAC, 44100, 16, 1),
    Format::new(32, 0x100000000, CODEC_TYPE_AAC, 48000, 16, 1),
];

struct Format {
    index: u8,
    value: u32,

    codec_type: CodecType,
    sample_rate: u32,
    bitness: u32,
    channels: u8,
}

impl Format {
    const fn new(
        index: u8,
        value: u32,
        codec_type: CodecType,
        sample_rate: u32,
        bitness: u32,
        channels: u8,
    ) -> Self {
        Self {
            index,
            value,
            codec_type,
            sample_rate,
            bitness,
            channels,
        }
    }

    fn codec_parameters(&self) -> CodecParameters {
        let channels_layout = match self.channels {
            1 => Layout::Mono,
            2 => Layout::Stereo,
            _ => unreachable!("unsupported channels count"),
        };
        match self.codec_type {
            CODEC_TYPE_PCM_S16LE | CODEC_TYPE_PCM_S24LE | CODEC_TYPE_AAC => CodecParameters {
                codec: self.codec_type,
                sample_rate: Some(self.sample_rate),
                channel_layout: Some(channels_layout),
                bits_per_sample: Some(self.bitness),
                ..Default::default()
            },

            CODEC_TYPE_ALAC => todo!("implement magic cookie later"),
            CODEC_TYPE_OPUS => todo!("opus still unimplemented"),

            _ => unreachable!("unsupported codec type"),
        }
    }
}

fn detect_format_by_index_or_bit(index: Option<usize>, bit: Option<usize>) -> Option<&Format> {
    if index.is_none() && bit.is_none() {
        return None;
    }
}*/

use std::sync::{atomic::AtomicU64, Mutex};

use openssl::aes::AesKey;

#[derive(Debug)]
pub enum CodecFormat {
    ALAC {
        frame_len: u32,
        compatible_version: u8,
        bit_depth: u8,
        pb: u8,
        mb: u8,
        kb: u8,
        channels_num: u8,
        max_run: u16,
        max_frame_bytes: u32,
        avg_bit_rate: u32,
        sample_rate: u32,
    },
}

pub enum SessionWithState {
    Announced {
        codec: CodecFormat,
        aes_key: Vec<u8>,
        aes_iv: Vec<u8>,
    },
    Setup {

    }
}

//#[derive(Debug)]
pub struct Session {
    pub codec: CodecFormat,
    pub aes_key: AesKey,
    pub iv: Mutex<Vec<u8>>,

    rtp_start: AtomicU64,
    rtp_current: AtomicU64,
    rtp_end: AtomicU64,
}

impl Session {
    pub fn init(codec: CodecFormat, aes_key: AesKey, iv: Vec<u8>) -> Self {
        Self {
            codec,
            aes_key,
            iv: Mutex::new(iv),
            rtp_start: Default::default(),
            rtp_current: Default::default(),
            rtp_end: Default::default(),
        }
    }
}

use std::fmt::{self, Debug};

use crossbeam_utils::atomic::AtomicCell;

use crate::crypt::Decryptor;

pub struct ClientSession {
    pub sample_rate: u32,
    pub sample_size: u16,
    pub channels: u8,
    pub volume: AtomicCell<f32>,
    pub decryptor: AtomicCell<Option<Box<dyn Decryptor + Send + Sync>>>,
}

impl Default for ClientSession {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            sample_size: 16,
            channels: 2,
            volume: AtomicCell::default(),
            decryptor: AtomicCell::default(),
        }
    }
}

impl Debug for ClientSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientSession")
            .field("sample_rate", &self.sample_rate)
            .field("sample_size", &self.sample_size)
            .field("channels", &self.channels)
            .field("volume", &self.volume.load())
            // TODO : atomic box with option .field("decryption", if self.decryption.as_ptr)
            .finish()
    }
}

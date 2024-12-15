use aes::cipher::{KeyIvInit, StreamCipher};
use ring::digest;

use super::AesCtr128BE;

pub struct AesCipher {
    cipher: AesCtr128BE,
}

impl AesCipher {
    pub fn new(
        fp_key: impl AsRef<[u8]>,
        shared_secret: impl AsRef<[u8]>,
        stream_connection_id: u64,
    ) -> Self {
        let key = {
            let mut digest = digest::Context::new(&digest::SHA512);
            digest.update(fp_key.as_ref());
            digest.update(shared_secret.as_ref());
            digest.finish()
        };

        let hash1 = {
            let mut digest = digest::Context::new(&digest::SHA512);
            digest.update(format!("AirPlayStreamKey{}", stream_connection_id).as_bytes());
            digest.update(&key.as_ref()[..16]);
            digest.finish()
        };

        let hash2 = {
            let mut digest = digest::Context::new(&digest::SHA512);
            digest.update(format!("AirPlayStreamIV{}", stream_connection_id).as_bytes());
            digest.update(&key.as_ref()[..16]);
            digest.finish()
        };

        Self {
            cipher: AesCtr128BE::new(hash1.as_ref()[..16].into(), hash2.as_ref()[..16].into()),
        }
    }

    pub fn open_in_place(&mut self, input: &mut [u8]) {
        self.cipher.apply_keystream(input)
    }
}

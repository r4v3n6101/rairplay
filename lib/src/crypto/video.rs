use aes::cipher::{KeyIvInit, StreamCipher};
use ring::digest;

use super::AesCtr128BE;

pub struct Cipher {
    aesctr: AesCtr128BE,
    og: [u8; 16],
    next_decrypt_count: usize,
}

impl Cipher {
    pub fn new(
        fp_key: impl AsRef<[u8]>,
        shared_secret: impl AsRef<[u8]>,
        stream_connection_id: i64,
    ) -> Self {
        let key = {
            let mut digest = digest::Context::new(&digest::SHA512);
            digest.update(fp_key.as_ref());
            digest.update(shared_secret.as_ref());
            digest.finish()
        };

        let hash1 = {
            let mut digest = digest::Context::new(&digest::SHA512);
            digest.update(format!("AirPlayStreamKey{stream_connection_id}").as_bytes());
            digest.update(&key.as_ref()[..16]);
            digest.finish()
        };

        let hash2 = {
            let mut digest = digest::Context::new(&digest::SHA512);
            digest.update(format!("AirPlayStreamIV{stream_connection_id}").as_bytes());
            digest.update(&key.as_ref()[..16]);
            digest.finish()
        };

        Self {
            aesctr: AesCtr128BE::new(hash1.as_ref()[..16].into(), hash2.as_ref()[..16].into()),
            og: [0; 16],
            next_decrypt_count: 0,
        }
    }

    pub fn decrypt(&mut self, inout: &mut [u8]) {
        let n = self.next_decrypt_count;

        // Step 1: Process leftover bytes from the previous call
        if n > 0 {
            for (i, x) in inout.iter_mut().enumerate().take(n) {
                *x ^= self.og[(16 - n) + i];
            }
        }

        // Step 2: Decrypt full blocks
        let encryptlen = ((inout.len() - n) / 16) * 16;
        self.aesctr.apply_keystream(&mut inout[n..n + encryptlen]);

        // Step 3: Handle remaining partial block
        let restlen = (inout.len() - n) % 16;
        let reststart = inout.len() - restlen;
        self.next_decrypt_count = 0;

        if restlen > 0 {
            self.og.fill(0);
            self.og[..restlen].copy_from_slice(&inout[reststart..]);
            self.aesctr.apply_keystream(&mut self.og);
            inout[reststart..].copy_from_slice(&self.og[..restlen]);
            self.next_decrypt_count = 16 - restlen;
        }
    }
}

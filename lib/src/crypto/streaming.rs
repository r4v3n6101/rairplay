use aes::cipher::{KeyIvInit as _, StreamCipher as _};
use ring::{aead, digest};

use super::AesCtr128BE;

pub struct AudioBufferedCipher {
    key: aead::LessSafeKey,
}

impl AudioBufferedCipher {
    pub const KEY_LEN: usize = 32;
    pub const AAD_LEN: usize = 8;
    pub const TAG_LEN: usize = 16;
    pub const NONCE_LEN: usize = aead::NONCE_LEN;

    pub fn new(key: [u8; Self::KEY_LEN]) -> Self {
        Self {
            key: aead::LessSafeKey::new(
                aead::UnboundKey::new(&aead::CHACHA20_POLY1305, &key).expect("valid keylen"),
            ),
        }
    }

    pub fn open_in_place(
        &self,
        nonce: [u8; Self::NONCE_LEN],
        aad: [u8; Self::AAD_LEN],
        tag: [u8; Self::TAG_LEN],
        inout: &mut [u8],
    ) -> Result<(), &'static str> {
        self.key
            .open_in_place_separate_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(aad),
                aead::Tag::from(tag),
                inout,
                0..,
            )
            .map_err(|_| "can't decipher buffered")
            .map(|_| ())
    }
}

pub struct AudioRealtimeCipher {
    // TODO : AES
}

pub struct VideoCipher {
    aesctr: AesCtr128BE,
    og: [u8; 16],
    next_decrypt_count: usize,
}

impl VideoCipher {
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

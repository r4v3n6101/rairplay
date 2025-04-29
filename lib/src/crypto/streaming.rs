use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, KeyIvInit as _, StreamCipher as _};
use ring::{aead, digest};

use super::{AesCbc128, AesCtr128BE, AesIv128, AesKey128};

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
                aead::UnboundKey::new(&aead::CHACHA20_POLY1305, &key).unwrap(),
            ),
        }
    }

    pub fn open_in_place(
        &self,
        nonce: [u8; Self::NONCE_LEN],
        aad: [u8; Self::AAD_LEN],
        tag: [u8; Self::TAG_LEN],
        inout: &mut [u8],
    ) -> Result<(), ()> {
        self.key
            .open_in_place_separate_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(aad),
                aead::Tag::from(tag),
                inout,
                0..,
            )
            .map_err(|_| ())
            .map(|_| ())
    }
}

pub struct AudioRealtimeCipher {
    aescbc: AesCbc128,
}

impl AudioRealtimeCipher {
    pub fn new(key: AesKey128, eiv: AesIv128) -> Self {
        Self {
            aescbc: AesCbc128::new(&key.into(), eiv.as_ref().into()),
        }
    }

    pub fn decrypt(&self, buf: &mut [u8]) {
        let encrypted_len = buf.len() - (buf.len() % 16);
        let _ = self
            .aescbc
            .clone()
            .decrypt_padded_mut::<NoPadding>(&mut buf[..encrypted_len]);
    }
}

pub struct VideoCipher {
    aesctr: AesCtr128BE,
    og: [u8; 16],
    next_decrypt_count: usize,
}

impl VideoCipher {
    pub fn new(key: AesKey128, stream_connection_id: i64) -> Self {
        let hash1 = {
            let mut digest = digest::Context::new(&digest::SHA512);
            digest.update(format!("AirPlayStreamKey{stream_connection_id}").as_bytes());
            digest.update(&key);
            digest.finish()
        };

        let hash2 = {
            let mut digest = digest::Context::new(&digest::SHA512);
            digest.update(format!("AirPlayStreamIV{stream_connection_id}").as_bytes());
            digest.update(&key);
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

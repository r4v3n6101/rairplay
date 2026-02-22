use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce, aead::AeadInOut};
use rand::CryptoRng;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};

use super::super::dto::ErrorCode;
use crate::crypto::hkdf;

enum Inner {
    Init,
    Established {
        accessory_pubkey: PublicKey,
        device_pubkey: PublicKey,
        shared_secret: SharedSecret,
    },
}

pub struct State {
    inner: Inner,
}

impl State {
    pub fn new() -> Self {
        Self { inner: Inner::Init }
    }

    pub fn m1_m2<R, F>(
        &mut self,
        mut rand: R,
        device_pubkey: &[u8],
        accessory_id: &[u8],
        sign: F,
    ) -> Result<(Vec<u8>, Vec<u8>), ErrorCode>
    where
        R: CryptoRng,
        F: FnOnce(&[u8]) -> Vec<u8>,
    {
        let Ok(device_pubkey) = <[u8; _]>::try_from(device_pubkey) else {
            return Err(ErrorCode::Authentication);
        };
        let device_pubkey = PublicKey::from(device_pubkey);

        let ephemeral = EphemeralSecret::random_from_rng(&mut rand);
        let accessory_pubkey = PublicKey::from(&ephemeral);
        let shared_secret = ephemeral.diffie_hellman(&device_pubkey);

        self.inner = Inner::Established {
            accessory_pubkey,
            device_pubkey,
            shared_secret,
        };

        let mut accessory_info = Vec::with_capacity(
            accessory_pubkey.as_bytes().len() + accessory_id.len() + device_pubkey.as_bytes().len(),
        );
        accessory_info.extend_from_slice(accessory_pubkey.as_bytes());
        accessory_info.extend_from_slice(accessory_id);
        accessory_info.extend_from_slice(device_pubkey.as_bytes());

        Ok((accessory_pubkey.to_bytes().to_vec(), sign(&accessory_info)))
    }

    pub fn m1_m2_enc(&self, msg: &mut Vec<u8>) -> Result<(), ErrorCode> {
        const SALT: &[u8] = b"Pair-Verify-Encrypt-Salt";
        const INFO: &[u8] = b"Pair-Verify-Encrypt-Info";
        const NONCE: &[u8] = b"\0\0\0\0PV-Msg02";

        let Inner::Established { shared_secret, .. } = &self.inner else {
            return Err(ErrorCode::Busy);
        };

        let session_key = hkdf(shared_secret.as_bytes(), SALT, INFO);

        let cipher = ChaCha20Poly1305::new(&session_key.into());
        if cipher
            .encrypt_in_place(&Nonce::try_from(NONCE).unwrap(), &[], msg)
            .is_err()
        {
            return Err(ErrorCode::Authentication);
        }

        Ok(())
    }

    pub fn m3_m4_dec(&self, msg: &mut Vec<u8>) -> Result<(), ErrorCode> {
        const SALT: &[u8] = b"Pair-Verify-Encrypt-Salt";
        const INFO: &[u8] = b"Pair-Verify-Encrypt-Info";
        const NONCE: &[u8] = b"\0\0\0\0PV-Msg03";

        let Inner::Established { shared_secret, .. } = &self.inner else {
            return Err(ErrorCode::Busy);
        };

        let session_key = hkdf(shared_secret.as_bytes(), SALT, INFO);

        let cipher = ChaCha20Poly1305::new(&session_key.into());
        if cipher
            .decrypt_in_place(&Nonce::try_from(NONCE).unwrap(), &[], msg)
            .is_err()
        {
            return Err(ErrorCode::Authentication);
        }

        Ok(())
    }

    pub fn m3_m4<F>(
        &self,
        device_id: &[u8],
        device_signature: &[u8],
        verify: F,
    ) -> Result<[u8; 32], ErrorCode>
    where
        F: FnOnce(&[u8], &[u8]) -> bool,
    {
        let Inner::Established {
            accessory_pubkey,
            device_pubkey,
            shared_secret,
        } = &self.inner
        else {
            return Err(ErrorCode::Busy);
        };

        let mut device_info = Vec::with_capacity(
            device_pubkey.as_bytes().len() + device_id.len() + accessory_pubkey.as_bytes().len(),
        );
        device_info.extend_from_slice(device_pubkey.as_bytes());
        device_info.extend_from_slice(device_id);
        device_info.extend_from_slice(accessory_pubkey.as_bytes());

        if !verify(&device_info, device_signature) {
            return Err(ErrorCode::Authentication);
        }

        Ok(shared_secret.to_bytes())
    }
}

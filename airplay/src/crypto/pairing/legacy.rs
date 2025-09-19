use std::mem;

use aes::cipher::StreamCipher;
use ed25519_dalek::{ed25519::signature::SignerMut as _, Signature, SigningKey, VerifyingKey};
use rand::{CryptoRng, Rng, RngCore};
use thiserror::Error;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::crypto::cipher_with_hashed_aes_iv;

use super::super::AesCtr128BE;

pub const X25519_KEY_LEN: usize = 32;
pub const SIGNATURE_LENGTH: usize = 64;

pub type X25519Key = [u8; X25519_KEY_LEN];
pub type Ed25519Key = [u8; X25519_KEY_LEN];
pub type SharedSecret = [u8; 32];
pub type Response = [u8; X25519_KEY_LEN + SIGNATURE_LENGTH];

#[derive(Default)]
enum Inner {
    #[default]
    Empty,
    Established {
        verify_their: VerifyingKey,
        pubkey_their: PublicKey,
        pubkey_our: PublicKey,
        shared_secret: SharedSecret,
    },
    Verified {
        shared_secret: SharedSecret,
    },
}

pub struct State {
    state: Inner,
    signing_our: SigningKey,
}

impl State {
    pub fn from_signing_privkey(privkey: Ed25519Key) -> Self {
        Self {
            state: Inner::default(),
            signing_our: SigningKey::from_bytes(&privkey),
        }
    }

    pub fn verifying_key(&self) -> Ed25519Key {
        self.signing_our.verifying_key().to_bytes()
    }

    pub fn establish_agreement<R>(
        &mut self,
        pubkey_their: X25519Key,
        verify_their: Ed25519Key,
        mut rand: R,
    ) -> Result<Response, Error>
    where
        R: RngCore + CryptoRng,
    {
        let verify_their = VerifyingKey::from_bytes(&verify_their)
            .map_err(|_| Error::Cryptography("invalid verification key"))?;
        let pubkey_their = PublicKey::from(pubkey_their);

        // Workaround for old version of rand_core
        let ephemeral = {
            let buf: [u8; _] = rand.random();
            StaticSecret::from(buf)
        };
        let pubkey_our = PublicKey::from(&ephemeral);
        let shared_secret = ephemeral.diffie_hellman(&pubkey_their).to_bytes();

        let mut signature = {
            let mut buf = [0u8; 2 * X25519_KEY_LEN];
            buf[..X25519_KEY_LEN].copy_from_slice(pubkey_our.as_ref());
            buf[X25519_KEY_LEN..].copy_from_slice(pubkey_their.as_ref());

            self.signing_our.sign(&buf).to_bytes()
        };

        let mut cipher = cipher(&shared_secret);
        cipher.apply_keystream(&mut signature);

        let mut response = [0u8; X25519_KEY_LEN + SIGNATURE_LENGTH];
        response[..X25519_KEY_LEN].copy_from_slice(pubkey_our.as_ref());
        response[X25519_KEY_LEN..].copy_from_slice(&signature);

        self.state = Inner::Established {
            verify_their,
            pubkey_our,
            pubkey_their,
            shared_secret,
        };

        Ok(response)
    }

    pub fn verify_agreement(&mut self, mut signature: [u8; SIGNATURE_LENGTH]) -> Result<(), Error> {
        let Inner::Established {
            verify_their,
            pubkey_their,
            pubkey_our,
            shared_secret,
        } = mem::take(&mut self.state)
        else {
            return Err(Error::WrongState);
        };

        let mut cipher = cipher(&shared_secret);
        cipher.apply_keystream(&mut [0u8; SIGNATURE_LENGTH]);
        cipher.apply_keystream(&mut signature);

        let mut message = [0u8; 2 * X25519_KEY_LEN];
        message[..X25519_KEY_LEN].copy_from_slice(pubkey_their.as_ref());
        message[X25519_KEY_LEN..].copy_from_slice(pubkey_our.as_ref());

        verify_their
            .verify_strict(&message, &Signature::from_bytes(&signature))
            .map_err(|_| Error::Verification)
            .inspect(|()| {
                self.state = Inner::Verified { shared_secret };
            })
    }

    pub fn shared_secret(&self) -> Option<SharedSecret> {
        match self.state {
            // Who tf knows when second verify ain't called?
            Inner::Established { shared_secret, .. } => {
                tracing::warn!("computed shared secret isn't verified by counterparty");
                Some(shared_secret)
            }
            Inner::Verified { shared_secret } => Some(shared_secret),
            Inner::Empty => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid state")]
    WrongState,
    // TODO : move somewhen to general errors of cryptography
    #[error("cryptography: {0}")]
    Cryptography(&'static str),
    #[error("signature verification")]
    Verification,
}

fn cipher(shared_secret: &[u8]) -> AesCtr128BE {
    cipher_with_hashed_aes_iv(b"Pair-Verify-AES-Key", b"Pair-Verify-AES-IV", shared_secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recreate_from_privkey_seed() {
        const PRIVKEY: [u8; 32] = [
            153, 62, 61, 195, 68, 210, 33, 179, 119, 105, 98, 195, 181, 225, 238, 146, 135, 226,
            224, 74, 233, 172, 222, 140, 80, 52, 153, 66, 147, 209, 98, 170,
        ];
        const EXPECTED_ED25519_PUBKEY: [u8; 32] = [
            63, 87, 112, 234, 30, 34, 240, 218, 63, 236, 178, 92, 117, 7, 156, 75, 162, 206, 30,
            66, 95, 192, 248, 148, 39, 50, 209, 206, 19, 44, 105, 205,
        ];

        let state = super::State::from_signing_privkey(PRIVKEY);
        assert_eq!(EXPECTED_ED25519_PUBKEY, state.verifying_key());
    }

    #[test]
    fn test_aes_cipher() {
        let mut text = [0x20u8; 2 * X25519_KEY_LEN];
        let mut cipher = cipher(&[0x10u8; 2 * X25519_KEY_LEN]);
        cipher.apply_keystream(&mut text);

        let expected = [
            123, 55, 157, 154, 188, 223, 183, 11, 180, 99, 194, 189, 187, 243, 152, 174, 79, 213,
            219, 50, 189, 204, 61, 74, 230, 202, 189, 13, 196, 104, 37, 250, 172, 238, 25, 252,
            145, 100, 207, 87, 135, 86, 121, 21, 183, 195, 126, 107, 222, 192, 242, 95, 5, 133,
            234, 157, 230, 24, 69, 31, 111, 61, 138, 99,
        ];

        assert_eq!(expected, text);
    }
}

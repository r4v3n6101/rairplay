use std::mem;

use aes::cipher::{KeyIvInit, StreamCipher};
use ring::{
    agreement, digest,
    rand::{self},
    signature::{self, KeyPair},
};
use thiserror::Error;

use super::super::AesCtr128BE;

pub const X25519_KEY_LEN: usize = 32;
pub const SIGNATURE_LENGTH: usize = 64;

pub type X25519Key = [u8; X25519_KEY_LEN];
pub type Ed25519Key = [u8; X25519_KEY_LEN];
pub type SharedSecret = [u8; 32];

#[derive(Default)]
enum Inner {
    #[default]
    Empty,
    Established {
        verify_their: signature::UnparsedPublicKey<Ed25519Key>,
        pubkey_their: agreement::UnparsedPublicKey<X25519Key>,
        pubkey_our: agreement::PublicKey,
        shared_secret: SharedSecret,
    },
    Verified {
        shared_secret: SharedSecret,
    },
}

pub struct State {
    state: Inner,
    keypair: signature::Ed25519KeyPair,
}

impl State {
    pub fn from_signing_privkey(privkey: Ed25519Key) -> Self {
        Self {
            state: Inner::default(),
            keypair: signature::Ed25519KeyPair::from_seed_unchecked(&privkey).unwrap(),
        }
    }

    pub fn verifying_key(&self) -> Ed25519Key {
        self.keypair.public_key().as_ref().try_into().unwrap()
    }

    pub fn establish_agreement(
        &mut self,
        pubkey_their: X25519Key,
        verify_their: Ed25519Key,
    ) -> Result<[u8; X25519_KEY_LEN + SIGNATURE_LENGTH], Error> {
        let rng = rand::SystemRandom::new();
        let pubkey_their = agreement::UnparsedPublicKey::new(&agreement::X25519, pubkey_their);
        let verify_their = signature::UnparsedPublicKey::new(&signature::ED25519, verify_their);
        let privkey_our = agreement::EphemeralPrivateKey::generate(&agreement::X25519, &rng)
            .map_err(|_| Error::Cryptography("ECDH private key generation"))?;
        let pubkey_our = privkey_our
            .compute_public_key()
            .map_err(|_| Error::Cryptography("ECDH public key computation"))?;
        let shared_secret = agreement::agree_ephemeral(privkey_our, &pubkey_their, |x| {
            SharedSecret::try_from(x).unwrap()
        })
        .map_err(|_| Error::Cryptography("ECDH agreement"))?;

        let mut signature: [u8; SIGNATURE_LENGTH] = {
            let mut buf = [0u8; 2 * X25519_KEY_LEN];
            buf[..X25519_KEY_LEN].copy_from_slice(pubkey_our.as_ref());
            buf[X25519_KEY_LEN..].copy_from_slice(pubkey_their.as_ref());

            self.keypair.sign(&buf).as_ref().try_into().unwrap()
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
            .verify(&message, &signature)
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
    let mut aes = digest::Context::new(&digest::SHA512);
    aes.update(b"Pair-Verify-AES-Key");
    aes.update(shared_secret);
    let aes = aes.finish();

    let mut iv = digest::Context::new(&digest::SHA512);
    iv.update(b"Pair-Verify-AES-IV");
    iv.update(shared_secret);
    let iv = iv.finish();

    AesCtr128BE::new(aes.as_ref()[..16].into(), iv.as_ref()[..16].into())
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

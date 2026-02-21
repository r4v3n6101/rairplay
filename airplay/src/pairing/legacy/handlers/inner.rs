use std::mem;

use aes::cipher::{KeyIvInit as _, StreamCipher};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::CryptoRng;
use thiserror::Error;
use x25519_dalek::{EphemeralSecret, PublicKey};

use crate::crypto::sha512_two_step;

pub const X25519_KEY_LEN: usize = 32;
pub const SIGNATURE_LENGTH: usize = 64;

type SharedSecret = [u8; 32];
type Response = [u8; X25519_KEY_LEN + SIGNATURE_LENGTH];
type AesCtr128BE = ctr::Ctr128BE<aes::Aes128>;

#[allow(clippy::large_enum_variant)]
enum Inner {
    Init,
    Established {
        verify_their: VerifyingKey,
        pubkey_their: PublicKey,
        pubkey_our: PublicKey,
        shared_secret: SharedSecret,
    },
}

pub struct State {
    state: Inner,
    signing_our: SigningKey,
}

impl State {
    pub fn from_signing_privkey(privkey: &[u8]) -> Self {
        let privkey = <[u8; _]>::try_from(privkey).expect("32 byte key");
        Self {
            state: Inner::Init,
            signing_our: SigningKey::from_bytes(&privkey),
        }
    }

    pub fn verifying_key(&self) -> Vec<u8> {
        self.signing_our.verifying_key().as_bytes().to_vec()
    }

    pub fn establish_agreement<R>(
        &mut self,
        mut rand: R,
        pubkey_their: &[u8],
        verify_their: &[u8],
    ) -> Result<(Response, SharedSecret), Error>
    where
        R: CryptoRng,
    {
        let Ok(verify_their) = <[u8; _]>::try_from(verify_their) else {
            return Err(Error::Cryptography("invalid verify key length"));
        };
        let verify_their = VerifyingKey::from_bytes(&verify_their)
            .map_err(|_| Error::Cryptography("invalid verification key"))?;

        let Ok(pubkey_their) = <[u8; _]>::try_from(pubkey_their) else {
            return Err(Error::Cryptography("invalid pubkey length"));
        };
        let pubkey_their = PublicKey::from(pubkey_their);

        let ephemeral = EphemeralSecret::random_from_rng(&mut rand);
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

        Ok((response, shared_secret))
    }

    pub fn verify_agreement(&mut self, mut signature: [u8; SIGNATURE_LENGTH]) -> Result<(), Error> {
        let Inner::Established {
            verify_their,
            pubkey_their,
            pubkey_our,
            shared_secret,
        } = mem::replace(&mut self.state, Inner::Init)
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
                self.state = Inner::Init;
            })
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
    let aes = sha512_two_step(b"Pair-Verify-AES-Key", shared_secret);
    let iv = sha512_two_step(b"Pair-Verify-AES-IV", shared_secret);

    AesCtr128BE::new((&aes).into(), (&iv).into())
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

        let state = super::State::from_signing_privkey(&PRIVKEY);
        assert_eq!(EXPECTED_ED25519_PUBKEY, state.verifying_key().as_slice());
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

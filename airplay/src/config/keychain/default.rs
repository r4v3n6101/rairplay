use std::{collections::HashMap, sync::Mutex};

use ed25519_dalek::{Signature, SigningKey, VerifyingKey, ed25519::signature::Signer};

use super::Keychain;

// TODO : I don't like mixing in-memory keychain algorithm and crypto algorith, but whatever
pub struct DefaultKeychain {
    self_id: Vec<u8>,
    keypair: (SigningKey, VerifyingKey),
    trusted: Mutex<HashMap<Vec<u8>, VerifyingKey>>,
    limit: usize,
}

impl Default for DefaultKeychain {
    fn default() -> Self {
        let signing_key = SigningKey::from_bytes(&[5; 32]);
        let verifying_key = signing_key.verifying_key();
        Self {
            self_id: b"default_none".to_vec(),
            keypair: (signing_key, verifying_key),
            trusted: Mutex::default(),
            limit: 10,
        }
    }
}

impl Keychain for DefaultKeychain {
    fn id(&self) -> &[u8] {
        &self.self_id
    }

    fn pubkey(&self) -> &[u8] {
        self.keypair.1.as_bytes()
    }

    fn sign(&self, data: &[u8]) -> Vec<u8> {
        self.keypair.0.sign(data).to_vec()
    }

    fn trust(&self, id: &[u8], key: &[u8]) -> bool {
        let mut trusted = self.trusted.lock().unwrap();
        if trusted.len() < self.limit {
            let Ok(key) = key.try_into() else {
                return false;
            };
            let Ok(key) = VerifyingKey::from_bytes(key) else {
                return false;
            };

            trusted.insert(id.to_vec(), key);
            true
        } else {
            false
        }
    }

    fn verify(&self, id: &[u8], message: &[u8], signature: &[u8]) -> bool {
        let trusted = self.trusted.lock().unwrap();
        let Some(key) = trusted.get(id) else {
            return false;
        };
        let Ok(signature) = Signature::from_slice(signature) else {
            return false;
        };

        key.verify_strict(message, &signature).is_ok()
    }
}

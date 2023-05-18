use openssl::{
    aes::{aes_ige, AesKey},
    symm::Mode,
};

use super::Decryptor;

pub struct AesDecryptor {
    key: AesKey,
    iv: Vec<u8>,
}

impl AesDecryptor {
    pub(crate) fn new(key: Vec<u8>, iv: Vec<u8>) -> Result<Self, &'static str> {
        Ok(Self {
            key: AesKey::new_decrypt(&key).map_err(|_| "invalid key")?,
            iv,
        })
    }
}

impl Decryptor for AesDecryptor {
    fn decrypt(&mut self, input: &[u8], output: &mut [u8]) {
        aes_ige(input, output, &self.key, &mut self.iv, Mode::Decrypt);
    }
}

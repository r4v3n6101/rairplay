use openssl::{
    aes::{aes_ige, AesKey},
    symm::Mode,
};

pub struct Key {
    key: AesKey,
    iv: Vec<u8>,
}

impl Key {
    pub(crate) fn new(key: Vec<u8>, iv: Vec<u8>) -> Result<Self, &'static str> {
        Ok(Self {
            key: AesKey::new_decrypt(&key).map_err(|_| "invalid key")?,
            iv,
        })
    }

    pub fn decrypt(&mut self, data: &[u8]) -> Vec<u8> {
        // TODO : don't know how much bytes
        let mut out = vec![0; 1024];
        aes_ige(data, &mut out, &self.key, &mut self.iv, Mode::Decrypt);

        out
    }
}

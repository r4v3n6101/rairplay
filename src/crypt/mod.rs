pub mod aes;
pub mod rsa;
// TODO : fair-play aes

pub trait Decryptor {
    fn decrypt(&mut self, input: &[u8], output: &mut [u8]);
}

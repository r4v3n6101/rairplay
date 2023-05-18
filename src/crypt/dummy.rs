use super::Decryptor;

pub struct DummyDecryptor;

impl Decryptor for DummyDecryptor {
    fn decrypt(&mut self, input: &[u8], output: &mut [u8]) {
        output.copy_from_slice(input);
    }
}

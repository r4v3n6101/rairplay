pub mod fairplay;
pub mod pairing;
pub mod streaming;

type AesCtr128BE = ctr::Ctr128BE<aes::Aes128>;
type AesCbc128 = cbc::Decryptor<aes::Aes128>;

pub type AesKey128 = [u8; 16];
pub type AesIv128 = [u8; 16];

/// Additionally hash AES key with shared secret from pairing
pub fn hash_aes_key(aes_key: AesKey128, shared_secret: impl AsRef<[u8]>) -> AesKey128 {
    let mut digest = ring::digest::Context::new(&ring::digest::SHA512);
    digest.update(aes_key.as_ref());
    digest.update(shared_secret.as_ref());
    digest.finish().as_ref()[..16].try_into().unwrap()
}

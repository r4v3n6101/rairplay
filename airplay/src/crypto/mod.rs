use aes::cipher::KeyIvInit as _;
use hkdf::Hkdf;
use sha2::Sha512;

pub type AesCtr128BE = ctr::Ctr128BE<aes::Aes128>;
pub type AesCbc128 = cbc::Decryptor<aes::Aes128>;
pub type AesKey128 = [u8; 16];
pub type AesIv128 = [u8; 16];
pub type X25519Key = [u8; 32];
pub type Ed25519Key = [u8; 32];
pub type ChaCha20Poly1305Key = [u8; 32];
pub type HkdfOutput = [u8; 32];

/// Additionally hash AES key with shared secret from pairing
pub fn hash_aes_key(aes_key: AesKey128, shared_secret: &[u8]) -> AesKey128 {
    sha512_two_step(&aes_key, shared_secret)
}

pub fn cipher_with_hashed_aes_iv(key_text: &[u8], iv_text: &[u8], secret: &[u8]) -> AesCtr128BE {
    let aes = sha512_two_step(key_text, secret);
    let iv = sha512_two_step(iv_text, secret);

    AesCtr128BE::new((&aes).into(), (&iv).into())
}

pub fn hkdf(input: &[u8], salt: &[u8], info: &[u8]) -> HkdfOutput {
    let hkdf = Hkdf::<Sha512>::new(Some(salt), input);
    let mut output = [0u8; _];
    hkdf.expand(info, &mut output)
        .expect("OKM must be 32 bytes len");

    output
}

fn sha512_two_step(x: &[u8], y: &[u8]) -> [u8; 16] {
    use sha2::{Digest, Sha512};

    let mut hasher = Sha512::new();
    hasher.update(x);
    hasher.update(y);

    let result = hasher.finalize();
    *result
        .first_chunk()
        .expect("sha512 must return at least 64 elements")
}

#[cfg(test)]
mod tests {
    use super::{AesKey128, hash_aes_key};

    #[test]
    fn test_hashing_aes_key() {
        const AES_KEY: AesKey128 = [
            17, 163, 62, 83, 175, 58, 156, 44, 127, 24, 45, 76, 218, 57, 48, 167,
        ];
        const SHARED_SECRET: &[u8] = &[
            82, 232, 92, 1, 109, 15, 74, 129, 146, 24, 94, 233, 48, 147, 185, 179, 138, 47, 128,
            131, 28, 37, 167, 104, 191, 46, 199, 34, 133, 50, 104, 7,
        ];

        const OUTPUT: &[u8] = &[
            207, 98, 45, 14, 107, 21, 73, 116, 51, 155, 84, 183, 136, 89, 31, 161,
        ];

        assert_eq!(OUTPUT, hash_aes_key(AES_KEY, SHARED_SECRET));
    }
}

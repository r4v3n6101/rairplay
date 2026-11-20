pub type AesKey128 = [u8; 16];
pub type AesIv128 = [u8; 16];
pub type ChaCha20Poly1305Key = [u8; 32];

pub fn hkdf(input: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    use hkdf::Hkdf;
    use sha2::Sha512;

    let hkdf = Hkdf::<Sha512>::new(Some(salt), input);
    let mut output = [0u8; _];
    hkdf.expand(info, &mut output)
        .expect("OKM must be 32 bytes len");

    output
}

pub fn sha512_two_step(x: &[u8], y: &[u8]) -> [u8; 16] {
    use sha2::{Digest, Sha512};

    let mut hasher = Sha512::new();
    hasher.update(x);
    hasher.update(y);

    let result = hasher.finalize();
    *result
        .first_chunk()
        .expect("sha512 must return at least 64 elements")
}

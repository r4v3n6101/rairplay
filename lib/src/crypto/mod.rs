pub mod fairplay;
pub mod pairing;
pub mod streaming;

type AesCtr128BE = ctr::Ctr128BE<aes::Aes128>;
type AesCbc128 = cbc::Decryptor<aes::Aes128>;

pub mod fairplay;
pub mod pairing;
pub mod streaming;

type AesCtr128BE = ctr::Ctr128BE<aes::Aes128>;

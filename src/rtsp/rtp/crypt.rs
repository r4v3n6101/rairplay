use super::packet::RtpPacket;

pub type Result<T> = std::result::Result<T, &'static str>;

pub struct ChaCha20Poly1305Cipher {
    key: ring::aead::LessSafeKey,
}

impl ChaCha20Poly1305Cipher {
    pub fn new(key: &[u8]) -> self::Result<Self> {
        ring::aead::UnboundKey::new(&ring::aead::CHACHA20_POLY1305, key)
            .map_err(|_| "invalid aead key length")
            .map(ring::aead::LessSafeKey::new)
            .map(|key| Self { key })
    }

    pub fn decrypt_pkt<'pkt>(&self, pkt: &'pkt mut RtpPacket) -> self::Result<&'pkt mut [u8]> {
        self.key
            .open_in_place_separate_tag(
                ring::aead::Nonce::assume_unique_for_key(
                    pkt.padded_nonce::<{ ring::aead::NONCE_LEN }>(),
                ),
                ring::aead::Aad::from(pkt.aad()),
                ring::aead::Tag::from(pkt.tag()),
                pkt.payload_mut(),
                0..,
            )
            .map_err(|_| "failed to decrypt aead")
    }
}

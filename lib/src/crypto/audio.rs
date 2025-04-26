use ring::aead;

const KEY_LEN: usize = 32;
const AAD_LEN: usize = 8;
const TAG_LEN: usize = 16;

pub struct BufferedCipher {
    // TODO : chacha20-poly1305
    key: aead::LessSafeKey,
}

impl BufferedCipher {
    pub fn new(key: [u8; KEY_LEN]) -> Self {
        Self {
            key: aead::LessSafeKey::new(
                aead::UnboundKey::new(&aead::CHACHA20_POLY1305, &key).expect("valid keylen"),
            ),
        }
    }

    pub fn open_in_place(
        &self,
        nonce: [u8; aead::NONCE_LEN],
        aad: [u8; AAD_LEN],
        tag: [u8; TAG_LEN],
        inout: &mut [u8],
    ) -> Result<(), &'static str> {
        self.key
            .open_in_place_separate_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(aad),
                aead::Tag::from(tag),
                inout,
                0..,
            )
            .map_err(|_| "can't decipher buffered")
            .map(|_| ())
    }
}

pub struct RealtimeCipher {
    // TODO : AES
}

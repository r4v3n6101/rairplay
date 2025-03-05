use ring::aead;

pub struct BufferedCipher {
    // TODO : chacha20-poly1305
    key: aead::LessSafeKey,
}

impl BufferedCipher {
    pub fn new() {}

    pub fn open_in_place(&self, what: ()) {}
}

pub struct RealtimeCipher {
    // TODO : AES
}

use bytes::BytesMut;
use openssl::{
    aes::{aes_ige, AesKey},
    symm::Mode,
};
use tokio_util::codec::Decoder;

pub struct AesDecoder<I> {
    inner: I,
    key: AesKey,
    iv: Vec<u8>,
}

impl<I> AesDecoder<I> {
    pub fn new(inner: I, key: AesKey, iv: Vec<u8>) -> Self {
        Self { inner, key, iv }
    }
}

impl<I: Decoder> Decoder for AesDecoder<I> {
    type Item = I::Item;
    type Error = I::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let mut buf = src.split_to(src.len());
        let mut out = BytesMut::new();
        out.resize(buf.len(), 0);
        aes_ige(&buf, &mut out, &self.key, &mut self.iv, Mode::Decrypt);

        self.inner.decode(&mut out)
    }
}

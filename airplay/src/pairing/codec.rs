use std::io;

use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use super::{
    SharedSessionKey,
    homekit::codec::{HAPDecoder, HAPEncoder},
};

pub struct UpgradeableCodec<E, D> {
    inner_encoder: E,
    inner_decoder: D,
    hap_encoder: Option<HAPEncoder>,
    hap_decoder: Option<HAPDecoder>,
    decode_buf: BytesMut,
    session_key: SharedSessionKey,
}

impl<E, D> UpgradeableCodec<E, D> {
    pub fn new(inner_encoder: E, inner_decoder: D, session_key: SharedSessionKey) -> Self {
        Self {
            inner_encoder,
            inner_decoder,
            session_key,
            hap_encoder: None,
            hap_decoder: None,
            decode_buf: BytesMut::new(),
        }
    }
}

impl<E, D> Decoder for UpgradeableCodec<E, D>
where
    D: Decoder<Item = BytesMut, Error = io::Error>,
{
    type Item = BytesMut;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(hap_decoder) = &mut self.hap_decoder {
            loop {
                // Try decode gathered data
                if !self.decode_buf.is_empty() {
                    tracing::trace!(item=?self.decode_buf, len=%self.decode_buf.len(), "trying to decode gathered buf");
                    if let Some(item) = self.inner_decoder.decode(&mut self.decode_buf)? {
                        tracing::trace!("gathered buf decoded");
                        return Ok(Some(item));
                    }
                }

                match hap_decoder.decode(src) {
                    // Gather buffer
                    Ok(Some(result)) => self.decode_buf.unsplit(result),
                    other => return other,
                }
            }
        } else {
            self.inner_decoder.decode(src)
        }
    }
}

impl<T, E, D> Encoder<T> for UpgradeableCodec<E, D>
where
    T: AsRef<[u8]>,
    E: for<'a> Encoder<&'a [u8], Error = io::Error>,
{
    type Error = io::Error;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let item = item.as_ref();
        if let Some(hap_encoder) = &mut self.hap_encoder {
            let mut tmp = BytesMut::with_capacity(item.len());
            self.inner_encoder.encode(item, &mut tmp)?;

            hap_encoder.encode(tmp, dst)
        } else {
            if let Some(session_key) = self.session_key.read()
                && session_key.upgrade_channel
            {
                self.hap_encoder
                    .replace(HAPEncoder::new(session_key.key_material));
                self.hap_decoder
                    .replace(HAPDecoder::new(session_key.key_material));
            }

            self.inner_encoder.encode(item, dst)
        }
    }
}

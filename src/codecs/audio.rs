use std::io;

use bytes::BytesMut;
use symphonia_core::{
    audio::RawSampleBuffer, codecs::Decoder as SDecoder, errors::Error, formats::Packet,
};
use tokio_util::codec::Decoder;

pub struct AudioDecoder {
    decoder: Box<dyn SDecoder>,
}

impl Decoder for AudioDecoder {
    type Item = ();
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let packet = Packet::new_from_slice(0, 0, 0, src.split_to(src.len()).as_ref());
        let buf = match self.decoder.decode(&packet) {
            Ok(buf) => {
                // TODO : poor quality
                let mut sample_buffer: RawSampleBuffer<i16> =
                    RawSampleBuffer::new(buf.frames() as u64, buf.spec().clone());
                sample_buffer.copy_planar_ref(buf);

                sample_buffer
            }
            Err(Error::IoError(err)) => return Err(err),
            Err(other) => return Err(io::Error::new(io::ErrorKind::Other, other)),
        };

        todo!()
    }
}

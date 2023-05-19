use std::io;

use bytes::{Bytes, BytesMut, Buf};
use rtp_rs::{RtpPacketBuilder, RtpReader, RtpReaderError};
use tokio_util::codec::{Decoder, Encoder};

pub struct RtpPacket {
    pub ty: u8,
    pub marked: bool,
    pub payload: Bytes,
}

pub struct RtpCodec;

impl Encoder<RtpPacket> for RtpCodec {
    type Error = io::Error;

    fn encode(&mut self, item: RtpPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let rtp = RtpPacketBuilder::new()
            .payload_type(item.ty)
            .marked(item.marked)
            .payload(&item.payload);
        dst.reserve(rtp.target_length() - dst.len());
        rtp.build_into_unchecked(dst.as_mut());

        Ok(())
    }
}

impl Decoder for RtpCodec {
    type Item = RtpPacket;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let (ty, marked, payload_offset) = match RtpReader::new(src) {
            Ok(reader) => (
                reader.payload_type(),
                reader.mark(),
                reader.payload_offset(),
            ),
            Err(RtpReaderError::BufferTooShort(buf_len)) => {
                src.reserve(RtpReader::MIN_HEADER_LEN - buf_len);
                return Ok(None);
            }
            Err(RtpReaderError::HeadersTruncated { header_len, .. }) => {
                src.reserve(header_len);
                return Ok(None);
            }
            Err(_) => return Err(io::Error::new(io::ErrorKind::Other, "rtp parsing error")),
        };

        src.advance(payload_offset);
        Ok(Some(RtpPacket {
            ty,
            marked,
            payload: src.copy_to_bytes(src.len()),
        }))
    }
}

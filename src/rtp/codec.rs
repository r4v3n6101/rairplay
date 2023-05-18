use std::io;

use bytes::BytesMut;
use rtp_rs::{RtpPacketBuilder, RtpReader, RtpReaderError};
use tokio_util::codec::{Decoder, Encoder};

pub struct RtpPacket {
    pub ty: u8,
    pub marked: bool,
    pub payload: Vec<u8>,
}

pub struct RtpCodec;

impl Encoder<RtpPacket> for RtpCodec {
    type Error = io::Error;

    fn encode(&mut self, item: RtpPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let rtp = RtpPacketBuilder::new()
            .payload_type(item.ty)
            .marked(item.marked)
            .payload(&item.payload);
        dst.reserve(rtp.target_length());
        rtp.build_into_unchecked(dst.as_mut());

        Ok(())
    }
}

impl Decoder for RtpCodec {
    type Item = RtpPacket;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match RtpReader::new(src.split_to(src.len()).as_ref()) {
            Ok(reader) => Ok(Some(RtpPacket {
                ty: reader.payload_type(),
                marked: reader.mark(),
                payload: reader.payload().to_vec(),
            })),
            Err(RtpReaderError::BufferTooShort(buf_len)) => {
                src.reserve(RtpReader::MIN_HEADER_LEN - buf_len);
                Ok(None)
            }
            Err(RtpReaderError::HeadersTruncated { header_len, .. }) => {
                src.reserve(header_len);
                Ok(None)
            }
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "rtp parsing error")),
        }
    }
}

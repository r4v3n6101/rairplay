use std::io;

use asynchronous_codec::{Decoder, Encoder};
use bytes::{Bytes, BytesMut};

pub struct RtpPacket {
    pub header: (),
    pub payload: Bytes,
}

pub struct RtpCodec;

impl Encoder for RtpCodec {
    type Item = RtpPacket;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        todo!()
    }
}

impl Decoder for RtpCodec {
    type Item = RtpPacket;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        todo!()
    }
}

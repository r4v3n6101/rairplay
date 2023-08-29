use std::io;

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Encoder, Decoder};

const HEADER_LEN: usize = 12;

#[derive(Debug)]
pub struct RtpPacket<P> {
    pub ty: u8,
    pub marked: bool,
    pub seq_num: u16,
    pub payload: P,
}

#[derive(Debug, Clone)]
pub struct RtpCodec<I> {
    inner: I,
}

impl<P, I: Encoder<P>> Encoder<RtpPacket<P>> for RtpCodec<I> {
    type Error = I::Error;

    fn encode(&mut self, item: RtpPacket<P>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(HEADER_LEN);
        dst.put_u8(2 << 6);
        dst.put_u8(item.ty | if item.marked { 0b10000000 } else { 0 });
        dst.put_u16(item.seq_num);
        dst.put_u32(0);
        dst.put_u32(0);

        self.inner.encode(item.payload, dst)
    }
}

impl<I: Decoder> Decoder for RtpCodec<I> {
    type Item = RtpPacket<I::Item>;
    type Error = I::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < HEADER_LEN {
            src.reserve(HEADER_LEN - src.len());
            return Ok(None);
        }

        let first_byte = src.get_u8();
        if first_byte & 0b11000000 < (2 << 6) {
            return Err(io::Error::new(io::ErrorKind::Other, "invalid magic number").into());
        }

        let second_byte = src.get_u8();
        let marked = second_byte & 0b10000000 == 0b10000000;
        let ty = second_byte & 0b1111111;

        let seq_num = src.get_u16();
        let timestamp = src.get_u32();
        let ssrc = src.get_u32();

        let Some(payload) = self.inner.decode(src)? else {
            return Ok(None);
        };

        Ok(Some(RtpPacket {
            ty,
            marked,
            seq_num,
            payload,
        }))
    }
}

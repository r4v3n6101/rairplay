use std::io;

use asynchronous_codec::{Decoder, Encoder};
use bytes::{Buf, BufMut, BytesMut};
use rtsp_types::{Message, ParseError, Request, Response, WriteError};

pub type RtspRequest = Request<Vec<u8>>;
pub type RtspResponse = Response<Vec<u8>>;

pub struct RtspCodec;

impl Encoder for RtspCodec {
    type Item = RtspResponse;
    type Error = io::Error;

    fn encode(&mut self, item: RtspResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.write(&mut dst.writer()).map_err(|err| match err {
            WriteError::IoError(io_err) => io_err,
        })
    }
}

impl Decoder for RtspCodec {
    type Item = RtspRequest;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        match Message::parse(&src) {
            Ok((msg, _)) => {
                src.advance(src.len());
                match msg {
                    Message::Request(req) => Ok(Some(req)),
                    _ => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "wrong packet type",
                    )),
                }
            }
            Err(ParseError::Incomplete(needed)) => {
                if let Some(add_len) = needed {
                    src.reserve(add_len.get());
                }
                Ok(None)
            }
            Err(ParseError::Error) => {
                Err(io::Error::new(io::ErrorKind::InvalidData, "parse error"))
            }
        }
    }
}

use std::io;

use bytes::{BufMut, BytesMut};
use rtsp_types::{Message, ParseError, Request, Response, WriteError};
use tokio_util::codec::{Decoder, Encoder};

pub struct RtspCodec;

impl<B: AsRef<[u8]>> Encoder<Response<B>> for RtspCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Response<B>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.write(&mut dst.writer()).map_err(|err| match err {
            WriteError::IoError(io_err) => io_err,
        })
    }
}

impl Decoder for RtspCodec {
    type Item = Request<Vec<u8>>;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let abc = &src.split_to(src.len());
        println!("{}", String::from_utf8_lossy(abc));
        match Message::parse(abc) {
            Ok((msg, _)) => match msg {
                Message::Request(req) => Ok(Some(req)),
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "wrong packet type",
                )),
            },
            Err(ParseError::Incomplete) => Ok(None),
            Err(ParseError::Error) => {
                // TODO : it crashes here
                Err(io::Error::new(io::ErrorKind::InvalidData, "parse error"))
            }
        }
    }
}

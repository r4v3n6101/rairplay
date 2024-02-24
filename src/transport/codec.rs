use std::io;

use bytes::{BufMut, Bytes, BytesMut};
use httparse::{Error, Request, Response, Status, EMPTY_HEADER};
use hyper::Uri;
use tokio_util::codec::{Decoder, Encoder};
use tracing::trace;

const MAX_HEADERS: usize = 32;

const RTSP_PATH_PREFIX: &[u8] = b"/rtsp";
const RTSP_VERSION: &[u8] = b"RTSP/1.0";
const RTSP_VERSION_CRLF: &[u8] = b"RTSP/1.0\r\n";
const HTTP_VERSION: &[u8] = b"HTTP/1.1";
const HTTP_VERSION_CRLF: &[u8] = b"HTTP/1.1\r\n";
const CRLF: &[u8] = b"\r\n";

pub struct Rtsp2Http;

impl Decoder for Rtsp2Http {
    type Item = Bytes;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let mut non_http = false;
        loop {
            let mut headers = [EMPTY_HEADER; MAX_HEADERS];
            let mut request = Request::new(&mut headers);
            match request.parse(src) {
                Ok(Status::Complete(len)) => {
                    let content_len = request
                        .headers
                        .iter()
                        .find_map(|header| {
                            if header.name.eq_ignore_ascii_case("content-length") {
                                std::str::from_utf8(header.value).ok()?.parse().ok()
                            } else {
                                None
                            }
                        })
                        .unwrap_or(src.len() - len);
                    if content_len > src.len() - len {
                        src.reserve(content_len - (src.len() - len));
                        return Ok(None);
                    }

                    let path = request.path.unwrap();

                    // Should be enough to fulfill HTTP request and new header
                    let mut output = BytesMut::with_capacity(src.len() + path.len() + 32);

                    // Method
                    let method = request.method.unwrap();
                    output.put_slice(method.as_bytes());
                    output.put_u8(b' ');

                    // URI path
                    match (path.parse::<Uri>(), non_http) {
                        (Ok(rtsp_uri), true) => {
                            output.put_slice(RTSP_PATH_PREFIX);
                            if rtsp_uri.path() != "*" {
                                output.put_slice(rtsp_uri.path().as_bytes());
                            }
                        }
                        _ => {
                            output.put_slice(path.as_bytes());
                        }
                    }
                    output.put_u8(b' ');

                    // Version & proto (it's always HTTP/1.1)
                    output.put_slice(HTTP_VERSION_CRLF);

                    // Original uri, for debugging
                    if non_http {
                        output.put_slice(b"x-rtsp-uri: ");
                        output.put_slice(path.as_bytes());
                        output.put_slice(CRLF);
                    }

                    // Headers
                    for header in request.headers {
                        output.put_slice(header.name.as_bytes());
                        output.put_slice(b": ");
                        output.put_slice(header.value);
                        output.put_slice(CRLF);
                    }
                    output.put_slice(CRLF);

                    // Body (i.e. remaining bytes after head of request)
                    output.put_slice(&src[len..]);

                    trace!(
                        "built new request, size {}, original size is {}",
                        output.len(),
                        src.len()
                    );

                    // Empty the buffer, so the next frame can be pulled
                    src.clear();

                    return Ok(Some(output.freeze()));
                }
                Ok(Status::Partial) => return Ok(None),
                Err(err @ Error::Version) => {
                    if non_http {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, err));
                    }
                    non_http = true;
                }
                Err(err) => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, err));
                }
            }

            if non_http {
                let Some(pos) = src
                    .windows(RTSP_VERSION_CRLF.len())
                    .position(|bytes| bytes == RTSP_VERSION_CRLF)
                    else {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "neither HTTP nor RTSP/1.0",
                        ));
                    };

                // Replacing version with HTTP and trying to parse again
                src[pos..pos + RTSP_VERSION_CRLF.len()].copy_from_slice(HTTP_VERSION_CRLF);
                trace!("replaced version at {pos} position");
            }
        }
    }
}

impl<T: AsRef<[u8]>> Encoder<T> for Rtsp2Http {
    type Error = io::Error;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let item = item.as_ref();

        let mut headers = [EMPTY_HEADER; MAX_HEADERS];
        let mut response = Response::new(&mut headers);
        let len = match response.parse(item) {
            Ok(Status::Complete(len)) => len,
            Ok(Status::Partial) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "partial response",
                ));
            }
            Err(err) => {
                return Err(io::Error::new(io::ErrorKind::InvalidData, err));
            }
        };

        let mut rtsp_mode = false;
        if let Some(upgrade) = response
            .headers
            .iter_mut()
            .find(|h| h.name.eq_ignore_ascii_case("Upgrade"))
        {
            if upgrade.value == b"RTSP" {
                rtsp_mode = true;
            } else {
                // Unknown upgradeable
            }

            // Erase `upgrade` header
            *upgrade = EMPTY_HEADER;
        }

        // Must be enough
        dst.reserve(item.len());

        // Version and proto
        dst.put_slice(if rtsp_mode {
            RTSP_VERSION
        } else {
            HTTP_VERSION
        });

        // Status code
        dst.put_slice(format!(" {}", response.code.expect("code is mandatory")).as_bytes());

        // Reason word
        if let Some(reason) = response.reason {
            dst.put_slice(format!(" {reason}").as_bytes());
        }
        dst.put_slice(CRLF);

        // Headers
        for header in response.headers {
            if header != &EMPTY_HEADER {
                // TODO : is that necessary?
                if header.name.eq_ignore_ascii_case("cseq") {
                    dst.put_slice(b"CSeq");
                } else {
                    dst.put_slice(header.name.as_bytes());
                }
                dst.put_slice(b": ");
                dst.put_slice(header.value);
                dst.put_slice(CRLF);
            }
        }
        dst.put_slice(CRLF);

        // TODO : use Content-Length
        // Body
        dst.put_slice(&item[len..]);

        trace!(
            "built new response, size is {}, original size is {}",
            dst.len(),
            item.len()
        );

        Ok(())
    }
}

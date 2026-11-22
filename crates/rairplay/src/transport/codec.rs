use std::io;

use http::Uri;
use httparse::{EMPTY_HEADER, Request, Response, Status};
use tokio_util::{
    bytes::{BufMut, BytesMut},
    codec::{Decoder, Encoder},
};

const MAX_HEADERS: usize = 32;

const RTSP_VERSION: &[u8] = b"RTSP/1.0";
const RTSP_VERSION_CRLF: &[u8] = b"RTSP/1.0\r\n";
const HTTP_VERSION_CRLF: &[u8] = b"HTTP/1.1\r\n";
const CRLF: &[u8] = b"\r\n";

pub struct Rtsp2Http;

impl Decoder for Rtsp2Http {
    type Item = BytesMut;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let start = src.len() - src.trim_ascii_start().len();
        let Some(newline) = src[start..].iter().position(|&byte| byte == b'\n') else {
            return Ok(None);
        };

        let mut need_more = false;
        loop {
            let line = &src[..=start + newline];
            if let Some(pos) = line
                .windows(RTSP_VERSION_CRLF.len())
                .position(|bytes| bytes == RTSP_VERSION_CRLF)
            {
                // Replacing version with HTTP and trying to parse again
                src[pos..pos + RTSP_VERSION_CRLF.len()].copy_from_slice(HTTP_VERSION_CRLF);
                tracing::trace!("replaced version at {pos} position");
            } else if let Some(pos) = line
                .windows(HTTP_VERSION_CRLF.len())
                .position(|bytes| bytes == HTTP_VERSION_CRLF)
            {
                // Replace back rtsp, for another call of decode
                src[pos..pos + HTTP_VERSION_CRLF.len()].copy_from_slice(RTSP_VERSION_CRLF);
                tracing::trace!("replaced back version at {pos} position");
            }

            if need_more {
                return Ok(None);
            }

            let mut headers = [EMPTY_HEADER; MAX_HEADERS];
            let mut request = Request::new(&mut headers);
            match request.parse(src) {
                Ok(Status::Complete(len)) => {
                    let content_len = request
                        .headers
                        .iter()
                        .find(|header| header.name.eq_ignore_ascii_case("content-length"))
                        .map_or(Ok(0), |header| {
                            std::str::from_utf8(header.value)
                                .ok()
                                .filter(|s| !s.starts_with('+'))
                                .and_then(|s| s.parse().ok())
                                .ok_or(io::ErrorKind::InvalidData)
                        })?;
                    let frame_len = len
                        .checked_add(content_len)
                        .ok_or(io::ErrorKind::InvalidData)?;
                    if content_len > src.len() - len {
                        need_more = true;
                        continue;
                    }

                    let path = request.path.unwrap();

                    // Should be enough to fulfill HTTP request and new header
                    let mut output = BytesMut::with_capacity(frame_len);

                    // Method
                    let method = request.method.unwrap();
                    output.put_slice(method.as_bytes());
                    output.put_u8(b' ');

                    // URI path
                    match path.parse::<Uri>() {
                        Ok(rtsp_uri) => {
                            if rtsp_uri.path() != "*" {
                                output.put_slice(rtsp_uri.path().as_bytes());
                            }
                        }
                        _ => {
                            if let Some(stripped) = path.strip_prefix("rtsp://") {
                                if let Some(pos) = stripped.find('/') {
                                    output.put_slice(&stripped.as_bytes()[pos..]);
                                } else {
                                    output.put_slice(path.as_bytes());
                                }
                            } else {
                                output.put_slice(path.as_bytes());
                            }
                        }
                    }
                    output.put_u8(b' ');

                    // Version & proto (it's always HTTP/1.1)
                    output.put_slice(HTTP_VERSION_CRLF);

                    // Headers
                    for header in request.headers {
                        output.put_slice(header.name.as_bytes());
                        output.put_slice(b": ");
                        output.put_slice(header.value);
                        output.put_slice(CRLF);
                    }
                    output.put_slice(CRLF);

                    let src = src.split_to(frame_len);
                    // Body (i.e. remaining bytes after head of request)
                    output.put_slice(&src[len..]);

                    tracing::trace!(
                        "built new request, size {}, original size is {}",
                        output.len(),
                        src.len()
                    );

                    return Ok(Some(output));
                }
                Ok(Status::Partial) => {
                    need_more = true;
                }
                Err(err) => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, err));
                }
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

        // Must be enough
        dst.reserve(item.len());

        // Version and proto
        dst.put_slice(RTSP_VERSION);

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
                dst.put_slice(header.name.as_bytes());
                dst.put_slice(b": ");
                dst.put_slice(header.value);
                dst.put_slice(CRLF);
            }
        }
        dst.put_slice(CRLF);

        // TODO : use Content-Length
        // Body
        dst.put_slice(&item[len..]);

        tracing::trace!(
            "built new response, size is {}, original size is {}",
            dst.len(),
            item.len()
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio_util::{bytes::BytesMut, codec::Decoder};

    use super::Rtsp2Http;

    #[test]
    fn decode_partial_and_pipelined_requests() {
        let post =
            b"\r\nPOST / RTSP/1.0\r\nX: RTSP/1.0\r\ncontent-length: 12\r\n\r\n\0RTSP/1.0\r\n\xff";
        let http =
            b"POST / HTTP/1.1\r\nX: RTSP/1.0\r\ncontent-length: 12\r\n\r\n\0RTSP/1.0\r\n\xff";
        let tail = b"GET / RTSP/1.0\r\n\r\nGET";
        for split in 0..post.len() {
            let mut src = BytesMut::from(&post[..split]);
            assert!(Rtsp2Http.decode(&mut src).unwrap().is_none());
            src.extend_from_slice(&post[split..]);
            src.extend_from_slice(tail);
            assert_eq!(Rtsp2Http.decode(&mut src).unwrap().unwrap(), &http[..]);
            assert_eq!(&src[..], tail);
            let get = Rtsp2Http.decode(&mut src).unwrap().unwrap();
            assert_eq!(&get[..], b"GET / HTTP/1.1\r\n\r\n");
            assert_eq!(&src[..], b"GET");
        }
    }

    #[test]
    fn decode_content_length() {
        let max = usize::MAX.to_string();
        for value in ["", "1x", "+1", &max, &format!("{max}0")] {
            let request = format!("POST /feedback RTSP/1.0\r\nContent-Length: {value}\r\n\r\n");
            let mut src = BytesMut::from(request.as_bytes());
            let err = Rtsp2Http.decode(&mut src).unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{value:?}");
        }

        let mut src =
            BytesMut::from("GET / RTSP/1.0\r\nContent-Length: 0\r\nContent-Length: 1\r\n\r\n");
        assert!(Rtsp2Http.decode(&mut src).unwrap().is_some());
    }

    #[test]
    fn decode_rtsp_setup_ipv4_and_ipv6() {
        let src_ipv4 = "SETUP rtsp://192.168.1.32/10491381106460282020 RTSP/1.0\r\nContent-Length: 0\r\nContent-Type: application/x-apple-binary-plist\r\nCSeq: 6\r\nDACP-ID: A3F9647052546E53\r\nActive-Remote: 3633173181\r\nUser-Agent: AirPlay/675.4.1\r\n\r\n";
        let src_ipv6 = "SETUP rtsp://fe80::3032:2ff:fe42:7267/4308029329791076611 RTSP/1.0\r\nContent-Length: 0\r\nContent-Type: application/x-apple-binary-plist\r\nCSeq: 6\r\nDACP-ID: 974F76DCFEAD7ECC\r\nActive-Remote: 418710485\r\nUser-Agent: AirPlay/695.5.1\r\n\r\n";

        let mut decoder = Rtsp2Http;

        let mut buffer = BytesMut::from(src_ipv4.as_bytes());
        let decoded = decoder
            .decode(&mut buffer)
            .expect("decode ipv4 request")
            .expect("ipv4 request decoded");
        let decoded = std::str::from_utf8(&decoded).expect("utf8 ipv4 output");
        let expected_ipv4 = "SETUP /10491381106460282020 HTTP/1.1\r\nContent-Length: 0\r\nContent-Type: application/x-apple-binary-plist\r\nCSeq: 6\r\nDACP-ID: A3F9647052546E53\r\nActive-Remote: 3633173181\r\nUser-Agent: AirPlay/675.4.1\r\n\r\n";
        assert_eq!(decoded, expected_ipv4);

        let mut buffer = BytesMut::from(src_ipv6.as_bytes());
        let decoded = decoder
            .decode(&mut buffer)
            .expect("decode ipv6 request")
            .expect("ipv6 request decoded");
        let decoded = std::str::from_utf8(&decoded).expect("utf8 ipv6 output");
        let expected_ipv6 = "SETUP /4308029329791076611 HTTP/1.1\r\nContent-Length: 0\r\nContent-Type: application/x-apple-binary-plist\r\nCSeq: 6\r\nDACP-ID: 974F76DCFEAD7ECC\r\nActive-Remote: 418710485\r\nUser-Agent: AirPlay/695.5.1\r\n\r\n";
        assert_eq!(decoded, expected_ipv6);
    }
}

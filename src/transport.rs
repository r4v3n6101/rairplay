use std::{
    io::{self},
    pin::Pin,
    task::{Context, Poll},
};

use axum::Router;
use bytes::{BufMut, Bytes, BytesMut};
use httparse::{Error as HError, Header, Request, Response, Status, EMPTY_HEADER};
use hyper::{body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use pin_project_lite::pin_project;
use tokio::{
    io::{split, ReadBuf},
    net::TcpListener,
};
use tokio_util::{
    codec::{Decoder, Encoder, FramedRead, FramedWrite},
    io::{SinkWriter, StreamReader},
};
use tower::Service;

const MAX_HEADERS: usize = 32;
const RTSP_VERSION: &[u8] = b"RTSP/1.0\r\n";
const HTTP_VERSION: &[u8] = b"HTTP/1.1\r\n";
const CRLF: &[u8] = b"\r\n";

const REMAPPED_RTSP_PATH: &[u8] = b"/rtsp";

pin_project! {
    struct IO<I, O> {
        #[pin]
        input: I,
        #[pin]
        output: O,
    }
}
impl<I: tokio::io::AsyncRead, O> tokio::io::AsyncRead for IO<I, O> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.project().input.poll_read(cx, buf)
    }
}
impl<I, O: tokio::io::AsyncWrite> tokio::io::AsyncWrite for IO<I, O> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().output.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().output.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().output.poll_shutdown(cx)
    }
}

pub async fn serve_with_rtsp_remap(tcp_listener: TcpListener, app: Router) {
    loop {
        let (socket, _remote_addr) = tcp_listener.accept().await.unwrap();

        let tower_service = app.clone();
        tokio::spawn(async move {
            let (rx, tx) = split(socket);
            let (rx, tx) = (
                StreamReader::new(FramedRead::new(rx, Rtsp2HttpCodec)),
                SinkWriter::new(FramedWrite::new(tx, Rtsp2HttpCodec)),
            );
            let unified = IO {
                input: rx,
                output: tx,
            };
            let io = TokioIo::new(unified);

            let hyper_service = service_fn(move |request: axum::http::Request<Incoming>| {
                tower_service.clone().call(request)
            });

            if let Err(err) = http1::Builder::new()
                .serve_connection(io, hyper_service)
                .await
            {
                eprintln!("failed to serve connection: {err:#}");
            }
        });
    }
}

struct Rtsp2HttpCodec;

impl Decoder for Rtsp2HttpCodec {
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
                    let uri = request.path.unwrap();

                    // Should be enough to fulfill HTTP request and new header
                    let mut output = BytesMut::with_capacity(src.len() + uri.len() + 32);

                    // Method
                    let method = request.method.unwrap();
                    output.put_slice(method.as_bytes());
                    output.put_u8(b' ');

                    // URI path
                    if non_http {
                        output.put_slice(REMAPPED_RTSP_PATH);
                    } else {
                        output.put_slice(uri.as_bytes());
                    }
                    output.put_u8(b' ');

                    // Version & proto (it's always HTTP ver. 1)
                    output.put_slice(HTTP_VERSION);

                    // Special headers for remapped RTSP
                    if non_http {
                        output.put_slice(b"Referer: ");
                        output.put_slice(uri.as_bytes());
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

                    // Empty the buffer, so the next frame can be pulled
                    src.clear();

                    return Ok(Some(output.freeze()));
                }
                Ok(Status::Partial) => return Ok(None),
                Err(err @ HError::Version) => {
                    if !non_http {
                        non_http = true;
                    } else {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, err));
                    }
                }
                Err(err) => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, err));
                }
            }

            if non_http {
                let Some(pos) = src
                    .windows(RTSP_VERSION.len())
                    .position(|bytes| bytes == RTSP_VERSION)
                else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "neither HTTP nor RTSP/1.0",
                    ));
                };

                // Replacing version with HTTP and trying to parse again
                src[pos..pos + RTSP_VERSION.len()].copy_from_slice(HTTP_VERSION);
            }
        }
    }
}

impl<T: AsRef<[u8]>> Encoder<T> for Rtsp2HttpCodec {
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
            dst.put_slice(format!(" {}", reason).as_bytes());
        }
        dst.put_slice(CRLF);

        // Headers
        for header in response.headers {
            if header != &EMPTY_HEADER {
                let Header { name, value } = header;
                dst.put_slice(name.as_bytes());
                dst.put_slice(b": ");
                dst.put_slice(value);
                dst.put_slice(CRLF);
            }
        }
        dst.put_slice(CRLF);

        // Body
        dst.put_slice(&item[len..]);

        Ok(())
    }
}

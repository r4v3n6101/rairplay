use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{BufMut, Bytes, BytesMut};
use httparse::{
    Error as HttparseError, Header, Request as HttparseRequest, Response as HttparseResponse,
    Status, EMPTY_HEADER,
};
use hyper::{
    body::{Body, Incoming},
    server::conn::http1,
    service::service_fn,
    Request, Response,
};
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
use tracing::{debug, error, trace};

const MAX_HEADERS: usize = 32;
const RTSP_VERSION: &[u8] = b"RTSP/1.0\r\n";
const HTTP_VERSION: &[u8] = b"HTTP/1.1\r\n";
const CRLF: &[u8] = b"\r\n";

const REMAPPED_RTSP_PATH: &[u8] = b"/rtsp";

type BoxStdError = Box<dyn std::error::Error + Send + Sync>;

pin_project! {
    struct RW<R, W> {
        #[pin]
        reader: R,
        #[pin]
        writer: W,
    }
}
impl<I: tokio::io::AsyncRead, O> tokio::io::AsyncRead for RW<I, O> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.project().reader.poll_read(cx, buf)
    }
}
impl<I, O: tokio::io::AsyncWrite> tokio::io::AsyncWrite for RW<I, O> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().writer.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().writer.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().writer.poll_shutdown(cx)
    }
}

pub async fn serve_with_rtsp_remap<B, S>(tcp_listener: TcpListener, svc: S)
where
    B: Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxStdError>,

    S: Clone + Send + 'static,
    S: Service<Request<Incoming>, Response = Response<B>>,
    S::Future: Send,
    S::Error: Into<BoxStdError>,
{
    loop {
        let (socket, remote_addr) = tcp_listener.accept().await.unwrap();
        debug!(%remote_addr, "got a new tcp connection");

        let tower_service = svc.clone();
        tokio::spawn(async move {
            let (rx, tx) = split(socket);
            let io = TokioIo::new(RW {
                reader: StreamReader::new(FramedRead::new(rx, Rtsp2HttpCodec)),
                writer: SinkWriter::new(FramedWrite::new(tx, Rtsp2HttpCodec)),
            });
            let hyper_service = service_fn(move |request| tower_service.clone().call(request));
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, hyper_service)
                .await
            {
                error!(%err, %remote_addr, "failed to serve connection");
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
            let mut request = HttparseRequest::new(&mut headers);
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
                Err(err @ HttparseError::Version) => {
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
                trace!("replaced version at {pos} position");
            }
        }
    }
}

impl<T: AsRef<[u8]>> Encoder<T> for Rtsp2HttpCodec {
    type Error = io::Error;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let item = item.as_ref();

        let mut headers = [EMPTY_HEADER; MAX_HEADERS];
        let mut response = HttparseResponse::new(&mut headers);
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

        trace!(
            "built new response, size is {}, original size is {}",
            dst.len(),
            item.len()
        );

        Ok(())
    }
}

use std::{
    io::{self},
    pin::Pin,
    task::{Context, Poll},
};

use axum::Router;
use bytes::{BufMut, Bytes, BytesMut};
use futures::TryStreamExt;
use httparse::{Header, Response, Status, EMPTY_HEADER};
use hyper::{body::Incoming, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server,
};
use pin_project_lite::pin_project;
use tokio::{
    io::{split, ReadBuf},
    net::TcpListener,
};
use tokio_util::{
    codec::{Decoder, Encoder, FramedRead, FramedWrite},
    compat::FuturesAsyncReadCompatExt,
    io::{CopyToBytes, SinkWriter},
};
use tower::Service;
use tracing::trace;

const RTSP_VERSION: &[u8] = b"RTSP/1.0";
const HTTP_VERSION: &[u8] = b"HTTP/1.1";

pub async fn serve_with_rtsp_remap(tcp_listener: TcpListener, app: Router) {
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
            let this = self.project();
            this.input.poll_read(cx, buf)
        }
    }
    impl<I, O: tokio::io::AsyncWrite> tokio::io::AsyncWrite for IO<I, O> {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, io::Error>> {
            let this = self.project();
            this.output.poll_write(cx, buf)
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
            let this = self.project();
            this.output.poll_flush(cx)
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), io::Error>> {
            let this = self.project();
            this.output.poll_shutdown(cx)
        }
    }

    loop {
        let (socket, _remote_addr) = tcp_listener.accept().await.unwrap();

        let tower_service = app.clone();
        tokio::spawn(async move {
            let (rx, tx) = split(socket);
            let (rx, tx) = (
                FramedRead::new(rx, Rtsp2HttpCodec)
                    .into_async_read()
                    .compat(),
                SinkWriter::new(CopyToBytes::new(FramedWrite::new(tx, Rtsp2HttpCodec))),
            );
            let unified = IO {
                input: rx,
                output: tx,
            };
            let socket = TokioIo::new(unified);

            let hyper_service = service_fn(move |request: axum::http::Request<Incoming>| {
                tower_service.clone().call(request)
            });

            if let Err(err) = server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(socket, hyper_service)
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

        let mut buf = src.split();
        if let Some(start_pos) = buf
            .windows(RTSP_VERSION.len())
            .position(|subslice| subslice == RTSP_VERSION)
        {
            trace!("replace RTSP header with HTTP");
            (&mut buf[start_pos..start_pos + HTTP_VERSION.len()]).copy_from_slice(HTTP_VERSION);
        }

        Ok(Some(buf.freeze()))
    }
}

impl Encoder<Bytes> for Rtsp2HttpCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut headers = [EMPTY_HEADER; 16];
        let mut response = Response::new(&mut headers);
        let len = match response.parse(&item) {
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
            if upgrade.value == RTSP_VERSION {
                rtsp_mode = true;
            } else {
                // Unknown upgradeable
            }

            // Erase `upgrade` header
            *upgrade = EMPTY_HEADER;
        }

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
        dst.put_slice(b"\r\n");

        // Headers
        for header in response.headers {
            if header != &EMPTY_HEADER {
                let Header { name, value } = header;
                dst.put_slice(name.as_bytes());
                dst.put_slice(b": ");
                dst.put_slice(value);
                dst.put_slice(b"\r\n");
            }
        }

        // Empty line
        dst.put_slice(b"\r\n");

        // Body
        dst.put_slice(&item[len..]);

        Ok(())
    }
}

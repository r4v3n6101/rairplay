use std::{convert::Infallible, error::Error, future::poll_fn, net::SocketAddr};

use hyper::{
    Request, Response,
    body::{Body, Incoming},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use tokio::{io::split, net::TcpListener};
use tokio_util::{
    codec::{FramedRead, FramedWrite},
    io::{SinkWriter, StreamReader},
};
use tower::Service;

mod codec;
mod util;

type BoxStdError = Box<dyn Error + Send + Sync>;

pub async fn serve_with_rtsp_remap<B, S, M>(tcp_listener: TcpListener, mut make_service: M)
where
    B: Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxStdError>,
    S: Service<Request<Incoming>, Response = Response<B>> + Clone + Send + 'static,
    S::Future: Send,
    S::Error: Into<BoxStdError>,
    M: Service<SocketAddr, Response = S, Error = Infallible>,
{
    loop {
        let local_addr = match tcp_listener.local_addr() {
            Ok(addr) => addr,
            Err(err) => {
                tracing::error!(%err, "couldn't get binding address");
                return;
            }
        };
        let (stream, remote_addr) = match tcp_listener.accept().await {
            Ok(res) => res,
            Err(err) => {
                tracing::error!(%err, "couldn't accept connection");
                continue;
            }
        };
        tracing::info!(%remote_addr, "got a new tcp connection");

        let _ = poll_fn(|cx| make_service.poll_ready(cx)).await;
        let Ok(tower_service) = make_service.call(local_addr).await;
        let hyper_service = service_fn(move |request| tower_service.clone().call(request));

        let (rx, tx) = split(stream);
        let io = TokioIo::new(util::RW {
            reader: StreamReader::new(FramedRead::new(rx, codec::Rtsp2Http)),
            writer: SinkWriter::new(FramedWrite::new(tx, codec::Rtsp2Http)),
        });

        tokio::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, hyper_service)
                .await
            {
                tracing::error!(%err, %remote_addr, "failed to serve connection");
            }
        });
    }
}

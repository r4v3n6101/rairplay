use std::{convert::Infallible, error::Error, future::poll_fn, net::SocketAddr, sync::Arc};

use axum::extract::connect_info::Connected;
use hyper::{
    body::{Body, Incoming},
    server::conn::http1,
    service::service_fn,
    Request, Response,
};
use hyper_util::rt::TokioIo;
use tokio::{io::split, net::TcpListener};
use tokio_util::{
    codec::{FramedRead, FramedWrite},
    io::{SinkWriter, StreamReader},
};
use tower::Service;

use crate::advertisment::AdvData;

mod codec;
mod util;

type BoxStdError = Box<dyn Error + Send + Sync>;

#[derive(Debug, Clone)]
pub struct IncomingStream {
    pub local_addr: Option<SocketAddr>,
    pub remote_addr: SocketAddr,
    pub adv_data: Arc<AdvData>,
}

impl Connected<IncomingStream> for IncomingStream {
    fn connect_info(target: IncomingStream) -> Self {
        target
    }
}

pub async fn serve_with_rtsp_remap<B, S, M>(
    tcp_listener: TcpListener,
    adv_data: Arc<AdvData>,
    mut make_service: M,
) where
    B: Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxStdError>,
    M: Service<IncomingStream, Response = S, Error = Infallible> + Clone + Send + 'static,
    S: Service<Request<Incoming>, Response = Response<B>> + Clone + Send + 'static,
    S::Future: Send,
    S::Error: Into<BoxStdError>,
{
    loop {
        let (stream, remote_addr) = match tcp_listener.accept().await {
            Ok(res) => res,
            Err(err) => {
                tracing::error!(%err, "couldn't accept connection");
                continue;
            }
        };
        tracing::debug!(%remote_addr, "got a new tcp connection");

        poll_fn(|cx| make_service.poll_ready(cx))
            .await
            .unwrap_or_else(|err| match err {});
        let tower_service = make_service
            .call(IncomingStream {
                local_addr: stream.local_addr().ok(),
                remote_addr,
                adv_data: Arc::clone(&adv_data),
            })
            .await
            .unwrap_or_else(|err| match err {});
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

use std::error::Error;

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
use tracing::{debug, error};

mod codec;
mod util;

type BoxStdError = Box<dyn Error + Send + Sync>;

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
            let io = TokioIo::new(util::RW {
                reader: StreamReader::new(FramedRead::new(rx, codec::Rtsp2Http)),
                writer: SinkWriter::new(FramedWrite::new(tx, codec::Rtsp2Http)),
            });
            let hyper_service = service_fn(move |request| tower_service.clone().call(request));
            if let Err(err) = http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, hyper_service)
                .await
            {
                error!(%err, %remote_addr, "failed to serve connection");
            }
        });
    }
}

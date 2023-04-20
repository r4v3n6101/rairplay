mod auth;
mod rtsp;

use futures::SinkExt;
use futures::StreamExt;
use tokio::net::TcpListener;
use tokio_util::codec::{FramedRead, FramedWrite};
use tower::ServiceBuilder;
use tower::ServiceExt;

use crate::rtsp::{codec::RtspCodec, RsaAuthLayer, RtspService};

#[tokio::main]
async fn main() {
    let tcp_listener = TcpListener::bind("0.0.0.0:8554").await.unwrap();

    while let Ok((stream, addr)) = tcp_listener.accept().await {
        println!("Got new connection: {}", addr);
        let (rx, tx) = stream.into_split();
        let (rx, tx) = (
            FramedRead::new(rx, RtspCodec),
            FramedWrite::new(tx, RtspCodec),
        );

        let svc = ServiceBuilder::new()
            .layer(RsaAuthLayer)
            .service(RtspService);

        svc.call_all(rx.filter_map(|res| async {
            match res {
                Ok(v) => Some(v),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    None
                }
            }
        }))
        .forward(tx.sink_err_into())
        .await
        .unwrap();
    }
}

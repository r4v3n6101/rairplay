mod auth;
mod rtsp;

use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::{FramedRead, FramedWrite};
use tower::{ServiceBuilder, ServiceExt};

#[tokio::main]
async fn main() {
    let tcp_listener = TcpListener::bind("0.0.0.0:8554").await.unwrap();
    println!("Binding to {}", tcp_listener.local_addr().unwrap());

    while let Ok((stream, addr)) = tcp_listener.accept().await {
        println!("Got new connection: {}", addr);
        let (rx, tx) = stream.into_split();
        let (rx, tx) = (
            FramedRead::new(rx, rtsp::Codec),
            FramedWrite::new(tx, rtsp::Codec),
        );

        ServiceBuilder::new()
            .layer(rtsp::RsaAuthLayer)
            .service(rtsp::Service::default())
            .call_all(rx.filter_map(|res| async {
                match res {
                    Ok(v) => Some(v),
                    Err(e) => {
                        println!("Error: {}", e);
                        None
                    }
                }
            }))
            .forward(tx.sink_err_into())
            .await
            .unwrap();
    }
}

mod crypt;
mod rtsp;

use std::net::Ipv4Addr;

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
            .layer(rtsp::RsaAuthLayer::new(
                Ipv4Addr::new(172, 20, 10, 6).into(),
                *b"\xA0\xDB\x0C\x69\xD3\x6F",
            ))
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

mod crypt;
mod rtp;
mod rtsp;
mod session;

use std::net::{Ipv4Addr, SocketAddr};

use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::{FramedRead, FramedWrite};
use tower::{ServiceBuilder, ServiceExt};
use tracing::Level;

use crate::rtsp::{codec::RtspCodec, layer::RsaAuthLayer, service::RtspService};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .init();

    let bind_addr: SocketAddr = "0.0.0.0:8554".parse().unwrap();
    let tcp_listener = TcpListener::bind(bind_addr).await.unwrap();
    println!("Binding to {}", tcp_listener.local_addr().unwrap());

    while let Ok((stream, client_addr)) = tcp_listener.accept().await {
        println!("Got new connection: {}", client_addr);
        let (rx, tx) = stream.into_split();
        let (rx, tx) = (
            FramedRead::new(rx, RtspCodec),
            FramedWrite::new(tx, RtspCodec),
        );

        ServiceBuilder::new()
            .layer(RsaAuthLayer::new(
                Ipv4Addr::new(172, 20, 10, 6).into(),
                *b"\xA0\xDB\x0C\x69\xD3\x6F",
            ))
            .service(RtspService::new(bind_addr.ip(), client_addr.ip()))
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

mod audio;
mod crypt;
mod rtp;
mod rtsp;

use std::{
    convert::Infallible,
    io::Write,
    net::{Ipv4Addr, SocketAddr},
    pin::Pin,
    task::{Context, Poll},
};

use audio::{AudioPacket, AudioSink};
use futures::{Sink, SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::{FramedRead, FramedWrite};
use tower::{ServiceBuilder, ServiceExt};
use tracing::Level;

use crate::rtsp::{codec::RtspCodec, layer::RsaAuthLayer, service::RtspService};

#[derive(Debug)]
struct Out;

impl Sink<AudioPacket> for Out {
    type Error = Infallible;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: AudioPacket) -> Result<(), Self::Error> {
        std::io::stderr().write_all(&item);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // no -op
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // no-op
        Poll::Ready(Ok(()))
    }
}

impl AudioSink for Out {
    fn initialize(sample_rate: u32, sample_size: u16, channels: u8) -> Self {
        Self
    }

    fn set_volume(&mut self, value: f32) {}

    fn get_volume(&self) -> f32 {
        0.0
    }
}

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
            .service(RtspService::<Out>::new(bind_addr.ip(), client_addr.ip()))
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

use std::net::SocketAddr;

use axum::serve::Listener;
use tokio::{
    io::Result,
    net::{TcpListener, TcpStream},
};
use tokio_util::{
    codec::{Decoder, Framed},
    io::{SinkWriter, StreamReader},
};

mod codec;

pub struct TcpListenerWithRtspRemap {
    pub tcp_listener: TcpListener,
}

impl Listener for TcpListenerWithRtspRemap {
    // TODO : TAIT upon it
    type Io = SinkWriter<
        StreamReader<Framed<TcpStream, codec::Rtsp2Http>, <codec::Rtsp2Http as Decoder>::Item>,
    >;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, remote_addr) = match self.tcp_listener.accept().await {
                Ok(res) => res,
                Err(err) => {
                    tracing::error!(%err, "couldn't accept connection");
                    continue;
                }
            };
            let io = SinkWriter::new(StreamReader::new(Framed::new(stream, codec::Rtsp2Http)));

            return (io, remote_addr);
        }
    }

    fn local_addr(&self) -> Result<Self::Addr> {
        self.tcp_listener.local_addr()
    }
}

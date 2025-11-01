use std::net::SocketAddr;

use axum::serve::Listener;
use tokio::{
    io::{Join, ReadHalf, Result, WriteHalf, join, split},
    net::{TcpListener, TcpStream},
};
use tokio_util::{
    codec::{Decoder, FramedRead, FramedWrite},
    io::{SinkWriter, StreamReader},
};

use crate::transport::codec::Rtsp2Http;

mod codec;

pub struct RtspListener {
    pub tcp_listener: TcpListener,
}

impl Listener for RtspListener {
    // TODO : TAIT upon it
    type Io = Join<
        StreamReader<FramedRead<ReadHalf<TcpStream>, Rtsp2Http>, <Rtsp2Http as Decoder>::Item>,
        SinkWriter<FramedWrite<WriteHalf<TcpStream>, Rtsp2Http>>,
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

            let (rx, tx) = split(stream);
            let io = join(
                StreamReader::new(FramedRead::new(rx, Rtsp2Http)),
                SinkWriter::new(FramedWrite::new(tx, Rtsp2Http)),
            );

            return (io, remote_addr);
        }
    }

    fn local_addr(&self) -> Result<Self::Addr> {
        self.tcp_listener.local_addr()
    }
}

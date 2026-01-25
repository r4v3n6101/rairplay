use std::{
    io,
    net::{SocketAddr, SocketAddrV4, SocketAddrV6},
};

use axum::serve::Listener;
use tokio::{io::Result, net::TcpStream};
use tokio_dual_stack::{DualStackTcpListener, Tcp as _};
use tokio_util::{
    codec::{Decoder, Framed},
    io::{SinkWriter, StreamReader},
};

mod codec;

pub struct TcpListenerWithRtspRemap {
    listener: DualStackTcpListener,
}

#[derive(Debug, Clone, Copy)]
pub enum ExtendedAddr {
    Local((SocketAddrV6, SocketAddrV4)),
    Peer {
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    },
}

impl TcpListenerWithRtspRemap {
    pub async fn bind(addr4: SocketAddrV4, addr6: SocketAddrV6) -> io::Result<Self> {
        Ok(Self {
            listener: DualStackTcpListener::bind(
                [SocketAddr::V4(addr4), SocketAddr::V6(addr6)].as_slice(),
            )
            .await?,
        })
    }
}

impl ExtendedAddr {
    pub fn local_addr(&self) -> SocketAddr {
        match *self {
            // Won't called, but pass v6 as default. Just for sure
            Self::Local((addr6, _)) => SocketAddr::V6(addr6),
            Self::Peer { local_addr, .. } => local_addr,
        }
    }
}

impl Listener for TcpListenerWithRtspRemap {
    // TODO : TAIT upon it
    // type Io = impl AsyncRead + AsyncWrite;
    type Io = SinkWriter<
        StreamReader<Framed<TcpStream, codec::Rtsp2Http>, <codec::Rtsp2Http as Decoder>::Item>,
    >;
    type Addr = ExtendedAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, remote_addr) = match self.listener.accept().await {
                Ok(res) => res,
                Err(err) => {
                    tracing::error!(%err, "couldn't accept connection");
                    continue;
                }
            };
            let local_addr = match stream.local_addr() {
                Ok(res) => res,
                Err(err) => {
                    tracing::error!(%err, "couldn't get local_addr of stream");
                    continue;
                }
            };

            return (
                SinkWriter::new(StreamReader::new(Framed::new(stream, codec::Rtsp2Http))),
                ExtendedAddr::Peer {
                    local_addr,
                    remote_addr,
                },
            );
        }
    }

    fn local_addr(&self) -> Result<Self::Addr> {
        self.listener.local_addr().map(ExtendedAddr::Local)
    }
}

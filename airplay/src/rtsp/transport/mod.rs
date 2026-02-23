use std::{
    io,
    net::{IpAddr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

use axum::serve::Listener;
use tokio::{io::Result, net::TcpStream};
use tokio_dual_stack::{DualStackTcpListener, Tcp as _};
use tokio_util::{
    codec::{Decoder, Framed},
    io::{SinkWriter, StreamReader},
};

use crate::pairing::{SharedSessionKey, codec::UpgradeableCodec};

mod codec;

pub struct TcpListenerWithRtspRemap {
    listener: DualStackTcpListener,
    bind_addr4: SocketAddrV4,
    bind_addr6: SocketAddrV6,
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub bind_addr4: SocketAddrV4,
    pub bind_addr6: SocketAddrV6,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub session_key: SharedSessionKey,
}

impl Connection {
    /// Returns an IP address of the same family as the given remote address
    pub fn bind_addr(&self) -> IpAddr {
        match self.remote_addr {
            SocketAddr::V4(_) => IpAddr::V4(*self.bind_addr4.ip()),
            SocketAddr::V6(_) => IpAddr::V6(*self.bind_addr6.ip()),
        }
    }
}

impl TcpListenerWithRtspRemap {
    pub async fn bind(addr4: SocketAddrV4, addr6: SocketAddrV6) -> io::Result<Self> {
        Ok(Self {
            listener: DualStackTcpListener::bind(
                [SocketAddr::V4(addr4), SocketAddr::V6(addr6)].as_slice(),
            )
            .await?,
            bind_addr4: addr4,
            bind_addr6: addr6,
        })
    }
}

impl Listener for TcpListenerWithRtspRemap {
    // TODO : TAIT upon it
    // type Io = impl AsyncRead + AsyncWrite;
    type Io = SinkWriter<
        StreamReader<
            Framed<TcpStream, UpgradeableCodec<codec::Rtsp2Http, codec::Rtsp2Http>>,
            <UpgradeableCodec<codec::Rtsp2Http, codec::Rtsp2Http> as Decoder>::Item,
        >,
    >;
    type Addr = Connection;

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
                    tracing::error!(%err, "couldn't get local addr of connection");
                    continue;
                }
            };

            let session_key = SharedSessionKey::default();
            return (
                SinkWriter::new(StreamReader::new(Framed::new(
                    stream,
                    UpgradeableCodec::new(codec::Rtsp2Http, codec::Rtsp2Http, session_key.clone()),
                ))),
                Connection {
                    session_key,
                    local_addr,
                    remote_addr,
                    bind_addr4: self.bind_addr4,
                    bind_addr6: self.bind_addr6,
                },
            );
        }
    }

    fn local_addr(&self) -> Result<Self::Addr> {
        unreachable!("you shall not pass")
    }
}

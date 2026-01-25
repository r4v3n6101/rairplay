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

mod codec;

pub struct TcpListenerWithRtspRemap {
    listener: DualStackTcpListener,
    bind_addr4: SocketAddrV4,
    bind_addr6: SocketAddrV6,
}

#[derive(Debug, Clone, Copy)]
pub struct Addresses {
    pub bind_addr4: SocketAddrV4,
    pub bind_addr6: SocketAddrV6,
    pub remote_addr: Option<SocketAddr>,
}

impl Addresses {
    /// Returns an IP address of the same family as the given remote address
    pub fn bind_addr(&self) -> Option<IpAddr> {
        self.remote_addr.map(|addr| match addr {
            SocketAddr::V4(_) => IpAddr::V4(*self.bind_addr4.ip()),
            SocketAddr::V6(_) => IpAddr::V6(*self.bind_addr6.ip()),
        })
    }

    pub fn remote_addr(&self) -> Option<IpAddr> {
        self.remote_addr.as_ref().map(SocketAddr::ip)
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
        StreamReader<Framed<TcpStream, codec::Rtsp2Http>, <codec::Rtsp2Http as Decoder>::Item>,
    >;
    type Addr = Addresses;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, remote_addr) = match self.listener.accept().await {
                Ok(res) => res,
                Err(err) => {
                    tracing::error!(%err, "couldn't accept connection");
                    continue;
                }
            };

            return (
                SinkWriter::new(StreamReader::new(Framed::new(stream, codec::Rtsp2Http))),
                Addresses {
                    bind_addr4: self.bind_addr4,
                    bind_addr6: self.bind_addr6,
                    remote_addr: Some(remote_addr),
                },
            );
        }
    }

    fn local_addr(&self) -> Result<Self::Addr> {
        self.listener
            .local_addr()
            .map(|(bind_addr6, bind_addr4)| Addresses {
                bind_addr4,
                bind_addr6,
                remote_addr: None,
            })
    }
}

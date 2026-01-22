use std::io;
use std::net::{IpAddr, SocketAddr, SocketAddrV6};

use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::{lookup_host, TcpListener, ToSocketAddrs, UdpSocket};

pub fn bind_addr(ip: IpAddr, port: u16) -> SocketAddr {
    match ip {
        IpAddr::V4(ipv4) => SocketAddr::new(IpAddr::V4(ipv4), port),
        IpAddr::V6(ipv6) => SocketAddr::new(IpAddr::V6(ipv6), port),
    }
}

pub async fn bind_tcp_dual_stack(bind_addr: impl ToSocketAddrs) -> io::Result<TcpListener> {
    let addr = select_bind_addr(bind_addr).await?;
    match addr {
        SocketAddr::V4(addr) => TcpListener::bind(addr).await,
        SocketAddr::V6(addr) => bind_tcp_ipv6_dual_stack(addr),
    }
}

pub async fn bind_udp_dual_stack(bind_addr: impl ToSocketAddrs) -> io::Result<UdpSocket> {
    let addr = select_bind_addr(bind_addr).await?;
    match addr {
        SocketAddr::V4(addr) => UdpSocket::bind(addr).await,
        SocketAddr::V6(addr) => bind_udp_ipv6_dual_stack(addr),
    }
}

async fn select_bind_addr(bind_addr: impl ToSocketAddrs) -> io::Result<SocketAddr> {
    let mut addrs = lookup_host(bind_addr).await?;
    let mut first = None;
    while let Some(addr) = addrs.next() {
        if addr.is_ipv6() {
            return Ok(addr);
        }
        if first.is_none() {
            first = Some(addr);
        }
    }

    first.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no bind address"))
}

fn bind_tcp_ipv6_dual_stack(addr: SocketAddrV6) -> io::Result<TcpListener> {
    let socket = Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_only_v6(false)?;
    socket.set_nonblocking(true)?;
    socket.bind(&addr.into())?;
    socket.listen(1024)?;
    TcpListener::from_std(socket.into())
}

fn bind_udp_ipv6_dual_stack(addr: SocketAddrV6) -> io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_only_v6(false)?;
    socket.set_nonblocking(true)?;
    socket.bind(&addr.into())?;
    UdpSocket::from_std(socket.into())
}

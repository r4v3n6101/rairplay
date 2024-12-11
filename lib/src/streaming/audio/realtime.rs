use std::{io, net::SocketAddr};

use tokio::net::{ToSocketAddrs, UdpSocket};

use crate::streaming::{
    audio::packet::{RtcpHeader, RtpHeader},
    command,
};

async fn data_processor(data_socket: UdpSocket) {
    const BUF_SIZE: usize = 16 * 1024;

    let mut buf = [0u8; BUF_SIZE];

    while let Ok(pkt_len) = data_socket.recv(&mut buf).await {
        let mut rtp_header = RtpHeader::empty();
        rtp_header.copy_from_slice(&buf[..RtpHeader::SIZE]);
    }
}

async fn control_processor(control_socket: UdpSocket) {
    const BUF_SIZE: usize = 16 * 1024;

    let mut buf = [0u8; BUF_SIZE];
    while let Ok(pkt_len) = control_socket.recv(&mut buf).await {
        let mut rtcp_header = RtcpHeader::empty();
        rtcp_header.copy_from_slice(&buf[..RtcpHeader::SIZE]);
    }
}

pub struct Channel {
    local_data_addr: SocketAddr,
    local_control_addr: SocketAddr,
}

impl Channel {
    pub async fn create(
        data_bind_addr: impl ToSocketAddrs,
        control_bind_addr: impl ToSocketAddrs,
        cmd_handler: command::Handler,
    ) -> io::Result<Self> {
        let data_socket = UdpSocket::bind(data_bind_addr).await?;
        let control_socket = UdpSocket::bind(control_bind_addr).await?;

        let local_data_addr = data_socket.local_addr()?;
        let local_control_addr = control_socket.local_addr()?;

        tokio::spawn(data_processor(data_socket));
        tokio::spawn(control_processor(control_socket));

        Ok(Channel {
            local_data_addr,
            local_control_addr,
        })
    }

    pub fn local_data_addr(&self) -> SocketAddr {
        self.local_data_addr
    }

    pub fn local_control_addr(&self) -> SocketAddr {
        self.local_control_addr
    }
}

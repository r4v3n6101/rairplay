use std::{
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, UdpSocket},
};

use crate::streaming::audio::packet::RtpPacket;

use super::buffer::AudioBuffer;

struct Inner {
    udp_socket: UdpSocket,
    tcp_listener: TcpListener,

    udp_addr: SocketAddr,
    tcp_addr: SocketAddr,
}

#[derive(Clone)]
pub struct StreamHandler {
    inner: Arc<Inner>,
}

impl StreamHandler {
    pub async fn create(bind_addr: IpAddr, udp_port: u16, tcp_port: u16) -> io::Result<Self> {
        let udp_socket = UdpSocket::bind(SocketAddr::new(bind_addr, udp_port)).await?;
        let tcp_listener = TcpListener::bind(SocketAddr::new(bind_addr, tcp_port)).await?;

        // It may be zero port, so request it from system for sure
        let udp_addr = udp_socket.local_addr()?;
        let tcp_addr = tcp_listener.local_addr()?;

        Ok(Self {
            inner: Arc::new(Inner {
                udp_socket,
                tcp_listener,

                udp_addr,
                tcp_addr,
            }),
        })
    }

    pub fn udp_addr(&self) -> SocketAddr {
        self.inner.udp_addr
    }

    pub fn tcp_addr(&self) -> SocketAddr {
        self.inner.tcp_addr
    }

    // TODO : rename
    // TODO : pass sink pipeline onto packets will go
    // TODO : return stream id and link it, so it can be deleted using this id
    pub fn new_buffered_stream(&self, audio_buf_size: usize) -> ! {
        let this = self.clone();
        let task = tokio::spawn(async move {
            let listener = &this.inner.tcp_listener;
            let local_addr = this.inner.tcp_addr;
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    tracing::info!(%local_addr, %remote_addr, "accepted new connection");
                    processor(stream, audio_buf_size).await;
                }
                Err(err) => {
                    tracing::warn!(%local_addr, %err, "failed to accept new connection");
                }
            }
        });

        todo!()
    }
}

async fn processor(mut stream: TcpStream, audio_buf_size: usize) {
    let mut audio_buf = AudioBuffer::with_capacity(audio_buf_size);
    while let Ok(pkt_len) = stream.read_u16().await {
        // 2 is pkt_len size itself
        let pkt_len: usize = pkt_len.saturating_sub(2).into();
        if pkt_len < RtpPacket::base_len() {
            tracing::warn!(%pkt_len, "malformed rtp packet");
            continue;
        }

        let payload_len = pkt_len - RtpPacket::base_len();

        let mut header = [0u8; RtpPacket::header_len()];
        let mut trailer = [0u8; RtpPacket::trailer_len()];
        let mut payload = audio_buf.allocate_buf(payload_len);

        if let Err(err) = stream.read_exact(&mut header).await {
            tracing::warn!(%err, %pkt_len, "failed to read rtp header");
            continue;
        };
        if let Err(err) = stream.read_exact(&mut payload).await {
            tracing::warn!(%err, %pkt_len, "failed to read rtp payload");
            continue;
        };
        if let Err(err) = stream.read_exact(&mut trailer).await {
            tracing::warn!(%err, %pkt_len, "failed to read rtp trailer");
            continue;
        };

        let rtp_pkt = RtpPacket::new(header, trailer, payload);
    }
}

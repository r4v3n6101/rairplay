use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::net::{ToSocketAddrs, UdpSocket};

use crate::{
    device::{BufferedData, DataCallback},
    streaming::audio::packet::{RtcpHeader, RtpHeader},
    util::{jitter, memory},
};

type SharedPacketBuf = Arc<Mutex<jitter::Buffer<()>>>;

pub struct Channel {
    local_data_addr: SocketAddr,
    local_control_addr: SocketAddr,
    pkt_buf: SharedPacketBuf,
}

impl Channel {
    pub async fn create(
        data_bind_addr: impl ToSocketAddrs,
        control_bind_addr: impl ToSocketAddrs,
        audio_buf_size: u32,
        min_depth: Duration,
        max_depth: Duration,
        sample_rate: u32,
    ) -> io::Result<Self> {
        let data_socket = UdpSocket::bind(data_bind_addr).await?;
        let control_socket = UdpSocket::bind(control_bind_addr).await?;

        let local_data_addr = data_socket.local_addr()?;
        let local_control_addr = control_socket.local_addr()?;

        let pkt_buf = Arc::new(Mutex::new(jitter::Buffer::new(min_depth, max_depth)));
        tokio::spawn(data_processor(
            data_socket,
            audio_buf_size,
            sample_rate,
            Arc::clone(&pkt_buf),
        ));
        tokio::spawn(control_processor(control_socket));

        Ok(Channel {
            local_data_addr,
            local_control_addr,
            pkt_buf,
        })
    }

    pub fn local_data_addr(&self) -> SocketAddr {
        self.local_data_addr
    }

    pub fn local_control_addr(&self) -> SocketAddr {
        self.local_control_addr
    }

    pub fn data_callback(&self) -> DataCallback<()> {
        let pkt_buf = Arc::clone(&self.pkt_buf);
        Box::new(move || {
            let output = pkt_buf.lock().unwrap().pop();
            BufferedData {
                wait_until_next: Some(output.wait_time),
                data: output.data,
            }
        })
    }
}

async fn data_processor(
    data_socket: UdpSocket,
    audio_buf_size: u32,
    sample_rate: u32,
    data_buf: SharedPacketBuf,
) {
    const PKT_BUF_SIZE: usize = 8 * 1024;

    let mut pkt_buf = [0u8; PKT_BUF_SIZE];
    let mut audio_buf = memory::BytesHunk::new(audio_buf_size as usize);
    // TODO : handle break cases
    loop {
        let Ok(pkt_len) = data_socket.recv(&mut pkt_buf).await else {
            break;
        };

        if pkt_len < RtpHeader::SIZE {
            tracing::warn!(%pkt_len, "malformed realtime rtp packet");
            continue;
        }

        let mut rtp_header = RtpHeader::empty();
        rtp_header.copy_from_slice(&pkt_buf[..RtpHeader::SIZE]);

        let mut buf = audio_buf.allocate_buf(pkt_len - RtpHeader::SIZE);
        buf.copy_from_slice(&pkt_buf[RtpHeader::SIZE..pkt_len]);

        data_buf.lock().unwrap().insert(
            rtp_header.seqnum().into(),
            (Duration::from_secs(rtp_header.timestamp().into()) / sample_rate).as_nanos(),
            (),
        );
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

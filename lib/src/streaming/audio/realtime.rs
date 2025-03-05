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

type SharedAudioBuf = Arc<Mutex<jitter::Buffer<()>>>;

pub struct Channel {
    local_data_addr: SocketAddr,
    local_control_addr: SocketAddr,
    data_buf: SharedAudioBuf,
}

impl Channel {
    pub async fn create(
        data_bind_addr: impl ToSocketAddrs,
        control_bind_addr: impl ToSocketAddrs,
        audio_buf_size: usize,
        min_depth: Duration,
        max_depth: Duration,
    ) -> io::Result<Self> {
        let data_socket = UdpSocket::bind(data_bind_addr).await?;
        let control_socket = UdpSocket::bind(control_bind_addr).await?;

        let local_data_addr = data_socket.local_addr()?;
        let local_control_addr = control_socket.local_addr()?;

        let data_buf = Arc::new(Mutex::new(jitter::Buffer::new(min_depth, max_depth)));
        let audio_buf = memory::BytesHunk::new(audio_buf_size);
        tokio::spawn(data_processor(
            data_socket,
            audio_buf,
            Arc::clone(&data_buf),
        ));
        tokio::spawn(control_processor(control_socket));

        Ok(Channel {
            local_data_addr,
            local_control_addr,
            data_buf,
        })
    }

    pub fn local_data_addr(&self) -> SocketAddr {
        self.local_data_addr
    }

    pub fn local_control_addr(&self) -> SocketAddr {
        self.local_control_addr
    }

    pub fn data_callback(&self) -> DataCallback<()> {
        let data_buf = Arc::clone(&self.data_buf);
        Box::new(move || {
            let output = data_buf.lock().unwrap().pop();
            BufferedData {
                wait_until_next: Some(output.wait_time),
                data: output.data,
            }
        })
    }
}

async fn data_processor(
    data_socket: UdpSocket,
    mut audio_buf: memory::BytesHunk,
    data_buf: SharedAudioBuf,
) {
    const PKT_BUF_SIZE: usize = 8 * 1024;

    let mut pkt_buf = [0u8; PKT_BUF_SIZE];
    while let Ok(pkt_len) = data_socket.recv(&mut pkt_buf).await {
        if pkt_len < RtpHeader::SIZE {
            tracing::warn!(%pkt_len, "malformed realtime rtp packet");
            continue;
        }

        let mut rtp_header = RtpHeader::empty();
        rtp_header.copy_from_slice(&pkt_buf[..RtpHeader::SIZE]);

        let mut buf = audio_buf.allocate_buf(pkt_len - RtpHeader::SIZE);
        buf.copy_from_slice(&pkt_buf[RtpHeader::SIZE..pkt_len]);

        // TODO : reduce locks
        data_buf.lock().unwrap().insert(
            rtp_header.seqnum().into(),
            // TODO : convert timestamp to ms
            rtp_header.timestamp() as u64 / 44100,
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

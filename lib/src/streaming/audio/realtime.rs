use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::net::UdpSocket;

use crate::util::{jitter, memory};

use super::packet::{RtcpHeader, RtpHeader};

#[derive(Clone)]
pub struct SharedPacketBuf {
    inner: Arc<Mutex<jitter::Buffer<()>>>,
}

impl SharedPacketBuf {
    pub fn new(min_depth: Duration, max_depth: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(jitter::Buffer::new(min_depth, max_depth))),
        }
    }

    pub fn pop(&self) -> (Vec<()>, Duration) {
        let jitter::Output { wait_time, data } = self.inner.lock().unwrap().pop();
        (data, wait_time)
    }
}

pub async fn data_processor(
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

        data_buf.inner.lock().unwrap().insert(
            rtp_header.seqnum().into(),
            (Duration::from_secs(rtp_header.timestamp().into()) / sample_rate).as_nanos(),
            (),
        );
    }
}

pub async fn control_processor(control_socket: UdpSocket) {
    const BUF_SIZE: usize = 16 * 1024;

    let mut buf = [0u8; BUF_SIZE];
    while let Ok(pkt_len) = control_socket.recv(&mut buf).await {
        let mut rtcp_header = RtcpHeader::empty();
        rtcp_header.copy_from_slice(&buf[..RtcpHeader::SIZE]);
    }
}

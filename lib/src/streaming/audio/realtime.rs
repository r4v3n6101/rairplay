use std::{io, sync::Mutex, time::Duration};

use tokio::net::UdpSocket;

use crate::util::{io::is_io_error_fine, jitter, memory};

use super::packet::{RtcpHeader, RtpHeader};

pub struct PacketBuf {
    inner: Mutex<jitter::Buffer<()>>,
}

impl PacketBuf {
    pub fn new(min_depth: Duration, max_depth: Duration) -> Self {
        Self {
            inner: Mutex::new(jitter::Buffer::new(min_depth, max_depth)),
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
    data_buf: &PacketBuf,
) -> io::Result<()> {
    const PKT_BUF_SIZE: usize = 8 * 1024;

    let mut pkt_buf = [0u8; PKT_BUF_SIZE];
    let mut audio_buf = memory::BytesHunk::new(audio_buf_size as usize);
    let result = loop {
        let pkt_len = match data_socket.recv(&mut pkt_buf).await {
            Ok(pkt_len) => pkt_len,
            Err(err) => break err,
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
    };

    if is_io_error_fine(&result) {
        Ok(())
    } else {
        Err(result)
    }
}

pub async fn control_processor(control_socket: UdpSocket) -> io::Result<()> {
    const BUF_SIZE: usize = 16 * 1024;

    let mut buf = [0u8; BUF_SIZE];
    let result = loop {
        let pkt_len = match control_socket.recv(&mut buf).await {
            Ok(pkt_len) => pkt_len,
            Err(err) => break err,
        };

        let mut rtcp_header = RtcpHeader::empty();
        rtcp_header.copy_from_slice(&buf[..RtcpHeader::SIZE]);
    };

    if is_io_error_fine(&result) {
        Ok(())
    } else {
        Err(result)
    }
}

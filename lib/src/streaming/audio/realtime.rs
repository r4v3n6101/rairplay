use std::{io, sync::Mutex, time::Duration};

use tokio::net::UdpSocket;
use want::Giver;

use crate::{
    device::AudioPacket,
    util::{io::is_io_error_fine, jitter, memory},
};

use super::packet::{RtcpHeader, RtpHeader};

pub struct Data {
    pub wait_time: Duration,
    pub data: Vec<AudioPacket>,
}

pub async fn data_processor(
    data_socket: UdpSocket,
    audio_buf_size: u32,
    sample_rate: u32,
    min_depth: Duration,
    max_depth: Duration,

    mut gv: Giver,
    output: &Mutex<Option<Data>>,
) -> io::Result<()> {
    const PKT_BUF_SIZE: usize = 8 * 1024;

    let mut pkt_buf = [0u8; PKT_BUF_SIZE];
    let mut audio_buf = memory::BytesHunk::new(audio_buf_size as usize);
    let mut pkts = jitter::Buffer::<AudioPacket>::new(min_depth, max_depth);
    let result = loop {
        tokio::select! {
            _ = gv.want() => {
                let (wait_time, data) = pkts.pop();
                output.lock().unwrap().replace(Data {
                    wait_time,
                    data
                });
                gv.give();
            },
            else => {
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

                pkts.insert(
                    rtp_header.seqnum().into(),
                    (Duration::from_secs(rtp_header.timestamp().into()) / sample_rate).as_nanos(),
                    AudioPacket,
                );
            }
        }
    };

    let data = pkts.pop_remaining();
    output.lock().unwrap().replace(Data {
        wait_time: Duration::ZERO,
        data,
    });
    gv.give();

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

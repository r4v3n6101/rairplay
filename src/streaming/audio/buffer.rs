use std::mem;

use bytes::BytesMut;

use super::packet::RtpPacket;

pub struct AudioBuffer {
    rtp_buf: Vec<RtpPacket>,
    audio_buf: BytesMut,
}

impl AudioBuffer {
    pub fn new() -> Self {
        // TODO : calculate rtp_buf size and guessed audio buf size
        Self {
            rtp_buf: Vec::with_capacity(100),
            audio_buf: BytesMut::new(),
        }
    }

    pub fn allocate_buf(&mut self, requested_len: usize) -> BytesMut {
        if self.audio_buf.capacity() < requested_len {
            if let rtp_buf_len @ 1.. = self.rtp_buf.len() {
                let avg_payload_len = self
                    .rtp_buf
                    .iter()
                    .map(|pkt| pkt.payload().len())
                    .sum::<usize>()
                    / rtp_buf_len;
                let guessed_cap =
                    avg_payload_len * (self.rtp_buf.capacity().saturating_sub(rtp_buf_len));

                tracing::info!(
                    "trying to reserve {} bytes, requsted: {}",
                    guessed_cap,
                    requested_len
                );
                self.audio_buf.reserve(guessed_cap.max(requested_len));
            } else {
                tracing::info!("just reserve {} bytes", requested_len);
                self.audio_buf.reserve(requested_len);
            }
        }

        if self.audio_buf.len() < requested_len {
            self.audio_buf.resize(requested_len, 0);
        }

        self.audio_buf.split_to(requested_len)
    }

    pub fn push_packet(&mut self, pkt: RtpPacket) {
        self.rtp_buf.push(pkt);
        tracing::info!(
            "pushed {}/{} rtps",
            self.rtp_buf.len(),
            self.rtp_buf.capacity()
        );
        if self.rtp_buf.len() == self.rtp_buf.capacity() {
            let rtp_buf_cap = self.rtp_buf.capacity();
            // TODO : get out this buffer
            let buf = mem::replace(&mut self.rtp_buf, Vec::with_capacity(rtp_buf_cap));
            self.audio_buf = BytesMut::new();
        }
    }
}

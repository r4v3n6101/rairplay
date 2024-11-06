use std::mem;

use bytes::BytesMut;

use super::packet::RtpPacket;

pub struct AudioBuffer {
    pkts: Vec<RtpPacket>,
    audio_buf: BytesMut,
}

impl AudioBuffer {
    pub fn new() -> Self {
        // TODO : calculate rtp_buf size and guessed audio buf size
        Self {
            pkts: Vec::with_capacity(100),
            audio_buf: BytesMut::new(),
        }
    }

    pub fn allocate_buf(&mut self, requested_len: usize) -> BytesMut {
        if self.audio_buf.capacity() < requested_len {
            if let pkts_len @ 1.. = self.pkts.len() {
                let avg_payload_len = self
                    .pkts
                    .iter()
                    .map(|pkt| pkt.payload().len())
                    .sum::<usize>()
                    / pkts_len;
                let guessed_cap = avg_payload_len * (self.pkts.capacity().saturating_sub(pkts_len));

                self.audio_buf.reserve(guessed_cap.max(requested_len));
            } else {
                self.audio_buf.reserve(requested_len);
            }
        }

        self.audio_buf.resize(requested_len, 0);
        self.audio_buf.split_to(requested_len)
    }

    pub fn push_packet(&mut self, pkt: RtpPacket) {
        self.pkts.push(pkt);
        if self.pkts.len() == self.pkts.capacity() {
            let pkts_cap = self.pkts.capacity();
            // TODO : get out this buffer
            let buf = mem::replace(&mut self.pkts, Vec::with_capacity(pkts_cap));
            self.audio_buf = BytesMut::new();
        }
    }
}

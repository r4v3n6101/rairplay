use bytes::BytesMut;

pub struct AudioBuffer {
    buf: BytesMut,
    avg_pkt_len: usize,
}

impl AudioBuffer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: BytesMut::with_capacity(capacity),
            avg_pkt_len: 0,
        }
    }

    pub fn allocate_buf(&mut self, requested_len: usize) -> BytesMut {
        if requested_len == 0 {
            return BytesMut::new();
        }

        if self.buf.capacity() < requested_len {
            if self.avg_pkt_len == 0 {
                self.avg_pkt_len = requested_len;
            } else {
                self.avg_pkt_len = self.avg_pkt_len.saturating_add(requested_len) / 2;
            }
            self.buf.reserve(self.avg_pkt_len);
        }

        self.buf.resize(requested_len, 0);
        self.buf.split_to(requested_len)
    }
}

use bytes::BytesMut;

pub struct BytesHunk {
    buf: BytesMut,
    size: usize,
}

impl BytesHunk {
    pub fn new(size: usize) -> Self {
        Self {
            buf: BytesMut::zeroed(size),
            size,
        }
    }

    pub fn allocate_buf(&mut self, requested_len: usize) -> BytesMut {
        if requested_len == 0 {
            return BytesMut::new();
        }

        if self.buf.len() < requested_len {
            assert!(self.size >= requested_len);
            self.buf = BytesMut::zeroed(self.size);
        }

        self.buf.split_to(requested_len)
    }
}

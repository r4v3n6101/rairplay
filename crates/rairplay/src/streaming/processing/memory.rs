use bytes::BytesMut;

pub struct BytesHunk {
    buf: BytesMut,
}

impl BytesHunk {
    pub fn new(size: usize) -> Self {
        Self {
            buf: BytesMut::with_capacity(size),
        }
    }

    pub fn allocate_buf(&mut self, requested_len: usize) -> BytesMut {
        if requested_len == 0 {
            return BytesMut::new();
        }

        self.buf.resize(requested_len, 0);
        self.buf.split_to(requested_len)
    }
}

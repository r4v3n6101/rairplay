use bytes::BytesMut;

pub struct ByteBuffer {
    buf: BytesMut,
    buf_size: usize,
}

impl ByteBuffer {
    pub fn new(buf_size: usize) -> Self {
        Self {
            buf: BytesMut::zeroed(buf_size),
            buf_size,
        }
    }

    pub fn allocate_buf(&mut self, requested_len: usize) -> BytesMut {
        if requested_len == 0 {
            return BytesMut::new();
        }

        if self.buf.len() < requested_len {
            assert!(self.buf_size >= requested_len);
            self.buf = BytesMut::zeroed(self.buf_size);
        }

        self.buf.split_to(requested_len)
    }
}

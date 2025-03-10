use std::time::Duration;

use tokio::{io::AsyncReadExt, net::TcpStream};

use crate::util::memory;

use super::packet::VideoHeader;

pub async fn processor(mut stream: TcpStream, video_buf_size: u32, latency: Duration) {
    let mut video_buf = memory::BytesHunk::new(video_buf_size as usize);
    loop {
        let mut header = VideoHeader::empty();
        if stream.read_exact(&mut *header).await.is_err() {
            break;
        }

        let mut payload = video_buf.allocate_buf(header.payload_len() as usize);
        if stream.read_exact(&mut payload).await.is_err() {
            break;
        }

        // TODO : decrypt video
    }
}

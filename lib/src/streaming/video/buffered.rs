use std::{io, time::Duration};

use tokio::{io::AsyncReadExt, net::TcpStream};

use crate::util::{io::is_io_error_fine, memory};

use super::packet::VideoHeader;

pub async fn processor(
    mut stream: TcpStream,
    video_buf_size: u32,
    latency: Duration,
) -> io::Result<()> {
    let mut video_buf = memory::BytesHunk::new(video_buf_size as usize);
    let result = loop {
        let mut header = VideoHeader::empty();
        match stream.read_exact(&mut *header).await {
            Ok(_) => {}
            Err(err) => break err,
        }

        let mut payload = video_buf.allocate_buf(header.payload_len() as usize);
        match stream.read_exact(&mut payload).await {
            Ok(_) => {}
            Err(err) => break err,
        }

        // TODO : push video packets
    };

    if is_io_error_fine(&result) {
        Ok(())
    } else {
        Err(result)
    }
}

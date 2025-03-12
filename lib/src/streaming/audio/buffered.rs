use std::io;

use tokio::{io::AsyncReadExt, net::TcpStream};

use crate::util::{io::is_io_error_fine, memory};

use super::packet::{RtpHeader, RtpTrailer};

pub async fn processor(mut stream: TcpStream, audio_buf_size: u32) -> io::Result<()> {
    let mut audio_buf = memory::BytesHunk::new(audio_buf_size as usize);

    let result = loop {
        let pkt_len = match stream.read_u16().await {
            Ok(pkt_len) => pkt_len,
            Err(err) => break err,
        };

        // 2 is pkt_len size itself
        let pkt_len: usize = pkt_len.saturating_sub(2).into();

        let Some(payload_len) = pkt_len.checked_sub(RtpHeader::SIZE + RtpTrailer::SIZE) else {
            tracing::warn!(%pkt_len, "malformed rtp packet");
            continue;
        };

        let mut header = RtpHeader::empty();
        let mut trailer = RtpTrailer::empty();
        let mut payload = audio_buf.allocate_buf(payload_len);

        match (
            stream.read_exact(&mut *header).await,
            stream.read_exact(&mut payload).await,
            stream.read_exact(&mut *trailer).await,
        ) {
            (Ok(_), Ok(_), Ok(_)) => {}
            (Err(err), _, _) | (_, Err(err), _) | (_, _, Err(err)) => break err,
        }
    };

    if is_io_error_fine(&result) {
        Ok(())
    } else {
        Err(result)
    }
}

use std::{io, net::SocketAddr};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, UdpSocket},
};

use crate::{
    crypto::streaming::{AudioBufferedCipher, AudioRealtimeCipher, VideoCipher},
    playback::{
        audio::{AudioPacket, AudioStream},
        video::{PacketKind, VideoPacket, VideoStream},
    },
    util::memory,
};

pub async fn event_processor(listener: TcpListener, local_addr: SocketAddr) {
    const BUF_SIZE: usize = 16 * 1024;

    let mut buf = [0; BUF_SIZE];
    while let Ok((mut stream, remote_addr)) = listener.accept().await {
        while let Ok(len @ 1..) = stream.read(&mut buf).await {
            tracing::debug!(%len, %remote_addr, %local_addr, "event data");
        }
    }
}

pub async fn audio_buffered_processor(
    audio_buf_size: u32,
    mut tcp_stream: TcpStream,
    cipher: AudioBufferedCipher,
    stream: &impl AudioStream,
) -> io::Result<()> {
    const TRAILER_LEN: usize = 24;

    let mut audio_buf = memory::BytesHunk::new(audio_buf_size as usize);

    loop {
        let pkt_len = tcp_stream.read_u16().await?;
        // 2 is pkt_len field size itself
        let pkt_len: usize = pkt_len.saturating_sub(2).into();

        if pkt_len < AudioPacket::HEADER_LEN + AudioPacket::TRAILER_LEN {
            return Err(io::Error::other("malformed buffered stream"));
        }

        // rtp pkt length w/o encryption data
        let pkt_len = pkt_len - TRAILER_LEN;
        let mut rtp = audio_buf.allocate_buf(pkt_len);
        tcp_stream.read_exact(&mut rtp).await?;

        let mut tag = [0u8; AudioBufferedCipher::TAG_LEN];
        let mut nonce = [0u8; AudioBufferedCipher::NONCE_LEN];
        let aad = (rtp.as_ref()[4..][..AudioBufferedCipher::AAD_LEN])
            .try_into()
            .unwrap();

        tcp_stream.read_exact(&mut tag).await?;
        tcp_stream.read_exact(&mut nonce[4..]).await?;

        // TODO : offload to thread pool
        if cipher
            .open_in_place(nonce, aad, tag, &mut rtp[AudioPacket::HEADER_LEN..])
            .is_err()
        {
            // TODO : logging
            continue;
        }

        stream.on_data(AudioPacket { rtp });
    }
}

pub async fn audio_realtime_processor(
    socket: UdpSocket,
    audio_buf_size: u32,
    cipher: AudioRealtimeCipher,
    stream: &impl AudioStream,
) -> io::Result<()> {
    const PKT_BUF_SIZE: usize = 16 * 1024;

    let mut pkt_buf = [0u8; PKT_BUF_SIZE];
    let mut audio_buf = memory::BytesHunk::new(audio_buf_size as usize);
    loop {
        let pkt_len = socket.recv(&mut pkt_buf).await?;

        if pkt_len < AudioPacket::HEADER_LEN {
            tracing::warn!(%pkt_len, "malformed realtime rtp packet");
            continue;
        }

        let mut rtp = audio_buf.allocate_buf(pkt_len);
        rtp.copy_from_slice(&pkt_buf[..pkt_len]);

        // TODO : offload data
        cipher.decrypt(&mut rtp[AudioPacket::HEADER_LEN..]);

        stream.on_data(AudioPacket { rtp });
    }
}

pub async fn control_processor(socket: UdpSocket) -> io::Result<()> {
    const BUF_SIZE: usize = 16 * 1024;

    let mut buf = [0u8; BUF_SIZE];
    loop {
        let _pkt_len = socket.recv(&mut buf).await?;
    }
}

pub async fn video_processor(
    video_buf_size: u32,
    mut tcp_stream: TcpStream,
    mut cipher: VideoCipher,
    stream: &impl VideoStream,
) -> io::Result<()> {
    const UNKNOWN_BYTES: usize = 112;

    let mut video_buf = memory::BytesHunk::new(video_buf_size as usize);
    loop {
        let payload_len = tcp_stream.read_u32_le().await?;
        let kind = match tcp_stream.read_u16_le().await? {
            1 => PacketKind::AvcC,
            0 | 4096 => PacketKind::Payload,
            other => PacketKind::Other(other),
        };
        let __unknown_field = tcp_stream.read_u16_le().await?;
        let timestamp = tcp_stream.read_u64_le().await?;

        tcp_stream.read_exact(&mut [0; UNKNOWN_BYTES]).await?;

        let mut pkt = VideoPacket {
            kind,
            timestamp,
            payload: video_buf.allocate_buf(payload_len as usize),
        };
        tcp_stream.read_exact(&mut pkt.payload).await?;

        // Header doesn't need decryption
        if matches!(kind, PacketKind::Payload) {
            // TODO : Offload to thread
            cipher.decrypt(&mut pkt.payload);
        }

        stream.on_data(pkt);
    }
}

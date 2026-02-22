use std::{io, net::IpAddr};

use bytes::Buf;
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, UdpSocket},
};
use tracing::Instrument;

use crate::{
    crypto::{AesIv128, AesKey128, ChaCha20Poly1305Key},
    pairing::SessionKey,
    playback::{
        audio::{AudioPacket, AudioStream},
        video::{PacketKind, VideoPacket, VideoStream},
    },
    streaming::processing::crypto::VideoCipher,
    util::memory,
};

mod crypto;

#[derive(Debug)]
pub enum Encryption {
    HomeKit { key: SessionKey },
    Legacy { key: AesKey128, iv: AesIv128 },
}

#[tracing::instrument(level = "DEBUG")]
pub async fn event_processor(listener: TcpListener) {
    const BUF_SIZE: usize = 16 * 1024;

    let mut buf = [0; BUF_SIZE];
    while let Ok((mut stream, remote_addr)) = listener.accept().await {
        while let Ok(len @ 1..) = stream.read(&mut buf).await {
            tracing::trace!(%len, %remote_addr, "event data");
        }
    }
}

#[tracing::instrument(level = "DEBUG", skip(stream))]
pub async fn audio_buffered_processor(
    audio_buf_size: u32,
    mut tcp_stream: TcpStream,
    key: ChaCha20Poly1305Key,
    stream: &impl AudioStream,
) -> io::Result<()> {
    const TRAILER_LEN: usize = 24;

    let mut audio_buf = memory::BytesHunk::new(audio_buf_size as usize);
    let cipher = crypto::AudioBufferedCipher::new(key);

    loop {
        async {
            let pkt_len = tcp_stream.read_u16().await?;
            // 2 is pkt_len field size itself
            let pkt_len: usize = pkt_len.saturating_sub(2).into();

            if pkt_len < AudioPacket::HEADER_LEN + TRAILER_LEN {
                return Err(io::Error::other("malformed buffered stream"));
            }

            // rtp pkt length w/o encryption data
            let pkt_len = pkt_len - TRAILER_LEN;
            let mut rtp = audio_buf.allocate_buf(pkt_len + TRAILER_LEN);
            tcp_stream.read_exact(&mut rtp).await?;
            tracing::trace!(%pkt_len, "packet read");

            if cipher.decrypt(&mut rtp).is_ok() {
                tracing::trace!("packet decrypted");

                stream.on_data(AudioPacket { rtp });
            } else {
                tracing::warn!("packet decryption failed");
            }
            tokio::task::consume_budget().await;

            Ok(())
        }
        .instrument(tracing::debug_span!("packet.buffered"))
        .await?;
    }
}

#[tracing::instrument(level = "DEBUG", skip(stream))]
pub async fn audio_realtime_processor(
    expected_remote_addr: IpAddr,
    socket: UdpSocket,
    audio_buf_size: u32,
    encryption: Encryption,
    stream: &impl AudioStream,
) -> io::Result<()> {
    let mut pkt_buf = [0u8; 16 * 1024];
    let mut audio_buf = memory::BytesHunk::new(audio_buf_size as usize);
    let cipher = match encryption {
        Encryption::HomeKit { .. } => {
            unimplemented!()
        }
        Encryption::Legacy { key, iv } => crypto::AudioRealtimeCipher::new(key, iv),
    };

    loop {
        async {
            let (pkt_len, remote_addr) = socket.recv_from(&mut pkt_buf).await?;

            // Filter out unexpected addresses
            if expected_remote_addr == remote_addr.ip() {
                if pkt_len < AudioPacket::HEADER_LEN {
                    tracing::warn!(%pkt_len, "malformed packet");
                } else {
                    let mut rtp = audio_buf.allocate_buf(pkt_len);
                    rtp.copy_from_slice(&pkt_buf[..pkt_len]);
                    tracing::trace!(%pkt_len, "packet read");

                    cipher.decrypt(&mut rtp[AudioPacket::HEADER_LEN..]);
                    tracing::trace!("packet decrypted");

                    stream.on_data(AudioPacket { rtp });
                    tokio::task::consume_budget().await;
                }
            } else {
                tracing::debug!(%remote_addr, "skip invalid connection");
            }

            io::Result::Ok(())
        }
        .instrument(tracing::debug_span!("packet.realtime"))
        .await?;
    }
}

#[tracing::instrument(level = "DEBUG", err)]
pub async fn control_processor(_expected_remote_addr: IpAddr, socket: UdpSocket) -> io::Result<()> {
    const BUF_SIZE: usize = 16 * 1024;

    let mut buf = [0u8; BUF_SIZE];
    loop {
        let _pkt_len = socket.recv(&mut buf).await?;
    }
}

#[tracing::instrument(level = "DEBUG", skip(stream))]
pub async fn video_processor(
    video_buf_size: u32,
    mut tcp_stream: TcpStream,
    encryption: Encryption,
    stream_connection_id: u64,
    stream: &impl VideoStream,
) -> io::Result<()> {
    let cipher: &mut (dyn VideoCipher + Send + Sync) = match encryption {
        Encryption::HomeKit { key } => {
            &mut crypto::HKVideoCipher::new(&key.key_material, stream_connection_id)
        }
        Encryption::Legacy { key, .. } => {
            &mut crypto::LegacyVideoCipher::new(key, stream_connection_id)
        }
    };

    let mut video_buf = memory::BytesHunk::new(video_buf_size as usize);
    loop {
        async {
            let mut header = [0u8; _];
            tcp_stream.read_exact(&mut header).await?;

            let mut ptr = &header[..];
            let payload_len = ptr.get_u32_le();
            let kind = match ptr.get_u16_le() {
                1 => PacketKind::AvcC,
                0 | 4096 => PacketKind::Payload,
                other => PacketKind::Other(other),
            };
            let unknown_field = ptr.get_u16_le();
            let timestamp = ptr.get_u64_le();

            let mut pkt = VideoPacket {
                kind,
                timestamp,
                payload: video_buf.allocate_buf(payload_len as usize),
            };
            tcp_stream.read_exact(&mut pkt.payload).await?;
            tracing::trace!(?kind, %timestamp, unknown=%unknown_field, %payload_len, "packet read");

            // Only payload need to be decrypted
            // TODO: Other(_) too?
            if matches!(kind, PacketKind::Payload) {
                if cipher.decrypt(header, &mut pkt.payload).is_ok() {
                    tracing::trace!("packet decrypted");
                } else {
                    tracing::warn!("packet decryption failed");
                }
            }

            stream.on_data(pkt);
            tokio::task::consume_budget().await;

            io::Result::Ok(())
        }
        .instrument(tracing::debug_span!("packet.video"))
        .await?;
    }
}

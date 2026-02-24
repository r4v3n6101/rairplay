use std::{io, net::IpAddr};

use bytes::Buf;
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, UdpSocket},
};
use tracing::Instrument;

use super::EncryptionMaterial;
use crate::{
    crypto::{AesIv128, AesKey128, ChaCha20Poly1305Key},
    pairing::SessionKey,
    playback::{
        audio::{AudioPacket, AudioStream},
        video::{PacketKind, VideoPacket, VideoStream},
    },
};

mod crypto;
mod memory;

#[derive(Debug)]
pub enum Encryption {
    ChaCha {
        key: ChaCha20Poly1305Key,
    },
    HomeKit {
        key: SessionKey,
        stream_connection_id: u64,
    },
    Legacy {
        key: AesKey128,
        iv: AesIv128,
        stream_connection_id: Option<u64>,
    },
}

impl TryFrom<EncryptionMaterial> for Encryption {
    type Error = io::Error;

    fn try_from(value: EncryptionMaterial) -> Result<Self, Self::Error> {
        if let Some(key) = value.chacha_key {
            Ok(Encryption::ChaCha { key })
        } else if let Some(key) = value.aeskey
            && let Some(iv) = value.aesiv
        {
            Ok(Encryption::Legacy {
                key,
                iv,
                stream_connection_id: value.stream_connection_id,
            })
        } else if let Some(key) = value.session_key
            && let Some(stream_connection_id) = value.stream_connection_id
        {
            Ok(Encryption::HomeKit {
                key,
                stream_connection_id,
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "no encryption key passed",
            ))
        }
    }
}

#[tracing::instrument(level = "DEBUG")]
pub async fn event_processor(listener: TcpListener) {
    let mut buf = [0; 16 * 1024];
    while let Ok((mut stream, remote_addr)) = listener.accept().await {
        while let Ok(len @ 1..) = stream.read(&mut buf).await {
            tracing::trace!(%len, %remote_addr, "event data");
        }
    }
}

#[tracing::instrument(level = "DEBUG", skip(stream))]
pub async fn audio_buffered_processor(
    mut tcp_stream: TcpStream,
    stream: &impl AudioStream,
    audio_buf_size: u32,
    encryption: Encryption,
) -> io::Result<()> {
    const TRAILER_LEN: usize = 24;

    let mut audio_buf = memory::BytesHunk::new(audio_buf_size as usize);
    let cipher = build_audio_cipher(&encryption);

    loop {
        async {
            let pkt_len = tcp_stream.read_u16().await?;
            // 2 is pkt_len field size itself
            let pkt_len: usize = pkt_len.saturating_sub(2).into();

            if pkt_len < AudioPacket::HEADER_LEN + TRAILER_LEN {
                return Err(io::Error::other("malformed buffered stream"));
            }

            let mut rtp = audio_buf.allocate_buf(pkt_len);
            tcp_stream.read_exact(&mut rtp).await?;
            tracing::trace!(%pkt_len, "packet read");

            if cipher.decrypt(&mut rtp).is_ok() {
                tracing::trace!("packet decrypted");
            } else {
                tracing::warn!("packet decryption failed");
            }

            stream.on_data(AudioPacket { rtp });
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
    stream: &impl AudioStream,
    audio_buf_size: u32,
    encryption: Encryption,
) -> io::Result<()> {
    let mut pkt_buf = [0u8; 16 * 1024];
    let mut audio_buf = memory::BytesHunk::new(audio_buf_size as usize);
    let cipher = build_audio_cipher(&encryption);

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

                    if cipher.decrypt(&mut rtp).is_ok() {
                        tracing::trace!("packet decrypted");
                    } else {
                        tracing::warn!("packet decryption failed");
                    }

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
    mut tcp_stream: TcpStream,
    stream: &impl VideoStream,
    video_buf_size: u32,
    encryption: Encryption,
) -> io::Result<()> {
    let mut video_buf = memory::BytesHunk::new(video_buf_size as usize);
    let mut cipher = build_video_cipher(&encryption);

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

fn build_audio_cipher(encryption: &Encryption) -> Box<dyn crypto::AudioCipher + Send + Sync> {
    match encryption {
        Encryption::ChaCha { key } => Box::new(crypto::ChachaAudioCipher::from_key(*key)),
        Encryption::HomeKit {
            key,
            stream_connection_id,
        } => Box::new(crypto::ChachaAudioCipher::from_secret_and_id(
            &key.key_material,
            *stream_connection_id,
        )),
        Encryption::Legacy { key, iv, .. } => Box::new(crypto::AesAudioCipher::new(*key, *iv)),
    }
}

fn build_video_cipher(encryption: &Encryption) -> Box<dyn crypto::VideoCipher + Send + Sync> {
    match encryption {
        Encryption::HomeKit {
            key,
            stream_connection_id,
        } => Box::new(crypto::ChachaVideoCipher::from_secret_and_id(
            &key.key_material,
            *stream_connection_id,
        )),
        Encryption::Legacy {
            key,
            stream_connection_id: Some(stream_connection_id),
            ..
        } => Box::new(crypto::AesVideoCipher::from_key_and_id(
            *key,
            *stream_connection_id,
        )),
        _ => unreachable!(),
    }
}

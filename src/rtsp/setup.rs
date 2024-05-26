use std::{
    fs::File,
    io::{self, Write},
    net::{IpAddr, SocketAddr},
};

use axum::extract::{ConnectInfo, State};
use bytes::BytesMut;
use futures::future::abortable;
use hyper::StatusCode;
use ring::aead;
use serde::{Deserialize, Serialize};
use symphonia::{
    core::{
        audio::{Channels, Layout, RawSampleBuffer},
        codecs::{
            CodecParameters, CodecRegistry, CodecType, Decoder, DecoderOptions, CODEC_TYPE_AAC,
        },
        formats::Packet,
    },
    default::codecs::AacDecoder,
};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, UdpSocket},
};

use crate::{plist::BinaryPlist, transport::IncomingStream};

use super::{
    dto::{SenderInfo, StreamDescriptor, StreamInfo, TimingPeerInfo},
    rtp::BufferedRtpPacket,
    state::{SenderHandle, SharedState, StreamHandle},
};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SetupRequest {
    SenderInfo(SenderInfo),
    Streams { streams: Vec<StreamInfo> },
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SetupResponse {
    Initial {
        #[serde(rename = "eventPort")]
        event_port: u16,
        #[serde(rename = "timingPort")]
        timing_port: u16,
        #[serde(rename = "timingPeerInfo")]
        timing_peer_info: Option<TimingPeerInfo>,
    },
    Streams {
        streams: Vec<StreamDescriptor>,
    },
}

pub async fn handler(
    State(SharedState { state, adv, .. }): State<SharedState>,
    ConnectInfo(IncomingStream {
        local_addr,
        remote_addr,
    }): ConnectInfo<IncomingStream>,
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> Result<BinaryPlist<SetupResponse>, StatusCode> {
    match req {
        SetupRequest::SenderInfo(info) => {
            let (event_listener, event_port) = match open_tcp(local_addr).await {
                Ok(res) => res,
                Err(err) => {
                    tracing::error!(%err, "failed to open event channel");
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            };
            let (event_task, event_handle) = abortable(event_handler(event_listener));

            let sender = SenderHandle { info, event_handle };
            // TODO : timing?
            tokio::spawn(event_task);
            tracing::info!(%event_port, "events handler spawned");

            // TODO : deal with timing
            let response = BinaryPlist(SetupResponse::Initial {
                event_port,
                timing_port: 0,
                timing_peer_info: Some(TimingPeerInfo {
                    id: adv.mac_addr.to_string(),
                    addresses: vec![local_addr],
                }),
            });
            *state.sender.write().unwrap() = Some(sender);

            Ok(response)
        }

        SetupRequest::Streams { streams } => {
            let mut streams_out = Vec::with_capacity(streams.len());
            for mut info in streams {
                // TODO : may be tcp or udp
                let (data_socket, local_data_port) = match open_tcp(local_addr /*, None*/).await {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::error!(%err, "failed to open data channel");
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                };
                let (data_task, data_handle) =
                    abortable(tcp_tracing(data_socket, info.shared_key.take().unwrap()));
                let (control_socket, local_control_port) = match open_udp(
                    local_addr,
                    info.remote_control_port
                        .map(|port| SocketAddr::new(remote_addr.ip(), port)),
                )
                .await
                {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::error!(%err, "failed to open control channel");
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                };
                let (control_task, control_handle) = abortable(udp_tracing(control_socket));

                let descriptor = StreamDescriptor {
                    audio_buffer_size: 8388608,

                    id: rand::random(),
                    ty: info.ty,

                    local_control_port,
                    local_data_port,
                };
                let handle = StreamHandle {
                    info,
                    descriptor,
                    data_handle,
                    control_handle,
                };
                tokio::spawn(data_task);
                tokio::spawn(control_task);
                tracing::info!(?handle, "new stream created");

                streams_out.push(handle.descriptor.clone());
                state.streams.write().unwrap().push(handle);
            }

            Ok(BinaryPlist(SetupResponse::Streams {
                streams: streams_out,
            }))
        }
    }
}

async fn open_tcp(local_addr: IpAddr) -> io::Result<(TcpListener, u16)> {
    let listener = TcpListener::bind((local_addr, 0)).await?;
    let port = listener.local_addr()?.port();

    Ok((listener, port))
}

async fn open_udp(
    local_addr: IpAddr,
    remote_addr: Option<SocketAddr>,
) -> io::Result<(UdpSocket, u16)> {
    let socket = UdpSocket::bind((local_addr, 0)).await?;
    if let Some(remote_addr) = remote_addr {
        socket.connect(remote_addr).await?;
    }
    let port = socket.local_addr()?.port();

    Ok((socket, port))
}

// TODO : this may be UDP
async fn tcp_tracing(listener: TcpListener, shk: BytesMut) {
    let unbound_key = aead::UnboundKey::new(&aead::CHACHA20_POLY1305, &shk).unwrap();
    let shared_key = aead::LessSafeKey::new(unbound_key);

    let mut codec = AacDecoder::try_new(
        &CodecParameters::new()
            .for_codec(CODEC_TYPE_AAC)
            .with_sample_rate(44100)
            .with_channel_layout(Layout::Stereo),
        &DecoderOptions::default(),
    )
    .unwrap();

    let mut file = File::create("raw.pcm").unwrap();
    match listener.accept().await {
        Ok((mut stream, remote_addr)) => {
            while let Ok(pkt_size) = stream.read_u16().await {
                // Decrease size itself
                let pkt_size = (pkt_size - 2) as usize;

                let mut pkt = BytesMut::zeroed(pkt_size);
                match stream.read_exact(&mut pkt).await {
                    Ok(_) => {
                        let mut buf_rtp = BufferedRtpPacket::new(pkt);
                        match shared_key.open_in_place_separate_tag(
                            aead::Nonce::assume_unique_for_key(
                                buf_rtp.padded_nonce::<{ aead::NONCE_LEN }>(),
                            ),
                            aead::Aad::from(buf_rtp.aad()),
                            aead::Tag::from(buf_rtp.tag()),
                            buf_rtp.rtp_mut().payload_mut(),
                            0..,
                        ) {
                            Ok(clear_data) => {
                                tracing::trace!(len = clear_data.len(), "unencrypted payload");

                                let packet = Packet::new_from_slice(0, 0, 0, clear_data);
                                match codec.decode(&packet) {
                                    Ok(audio_buf) => {
                                        tracing::info!(
                                            frames = audio_buf.frames(),
                                            "frames decoded"
                                        );

                                        let mut sample_buf = RawSampleBuffer::<i16>::new(
                                            audio_buf.capacity() as u64,
                                            *audio_buf.spec(),
                                        );
                                        sample_buf.copy_interleaved_ref(audio_buf);

                                        file.write(sample_buf.as_bytes()).unwrap();
                                    }
                                    Err(err) => {
                                        tracing::error!(%err, "(((");
                                    }
                                }
                            }
                            Err(err) => {
                                tracing::warn!(%err, "deciphering failed");
                            }
                        }
                    }
                    Err(err) => {
                        tracing::warn!(%err, "failed to read data packets");
                    }
                }
            }
        }
        Err(err) => {
            tracing::error!(%err, "data listener couldn't accept a connection");
        }
    }
}

async fn udp_tracing(socket: UdpSocket) {
    let mut buf = [0; 16 * 1024];
    while let Ok(len @ 1..) = socket.recv(&mut buf).await {
        tracing::debug!(%len, "control socket bytes");
    }
}

async fn event_handler(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((mut stream, remote_addr)) => {
                let mut buf = [0; 16 * 1024];
                while let Ok(len @ 1..) = stream.read(&mut buf).await {
                    tracing::debug!(%len, %remote_addr, "event stream bytes");
                }
            }
            Err(err) => {
                tracing::error!(%err, "event listener couldn't accept a connection");
            }
        }
    }
}

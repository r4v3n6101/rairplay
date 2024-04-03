use std::{
    future, io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use axum::extract::{ConnectInfo, State};
use bytes::Bytes;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, UdpSocket},
};
use tokio_util::sync::CancellationToken;

use crate::transport::IncomingStream;

use super::{
    plist::BinaryPlist,
    state::{SenderInfo, SharedState, Stream, StreamMetadata, StreamType, TimingProtocol},
};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SetupRequest {
    InfoEvent {
        #[serde(rename = "timingProtocol")]
        timing_protocol: String,
        #[serde(rename = "et")]
        encryption_ty: u8,
        #[serde(rename = "ekey")]
        encryption_key: Bytes,
        #[serde(rename = "eiv")]
        encryption_iv: Bytes,
        #[serde(rename = "deviceID")]
        device_id: String,
        #[serde(rename = "macAddress")]
        mac_addr: String,
        #[serde(rename = "osName")]
        os_name: Option<String>,
        #[serde(rename = "osVersion")]
        os_version: Option<String>,
        #[serde(rename = "osBuildVersion")]
        os_build_version: Option<String>,
        model: String,
        name: String,
    },
    DataControl {
        streams: Vec<StreamRequest>,
    },
}

#[derive(Debug, Deserialize)]
pub struct StreamRequest {
    #[serde(rename = "type")]
    ty: u8,
    #[serde(rename = "audioMode")]
    audio_mode: String,
    #[serde(rename = "ct")]
    compression_type: u8,
    #[serde(rename = "audioFormat")]
    audio_format: u32,
    #[serde(rename = "audioFormatIndex")]
    audio_format_index: Option<u32>,

    #[serde(rename = "latencyMin")]
    latency_min: Option<u32>,
    #[serde(rename = "latencyMax")]
    latency_max: Option<u32>,
    #[serde(rename = "spf")]
    samples_per_frame: u32,

    #[serde(rename = "controlPort")]
    control_port: Option<u16>,

    #[serde(rename = "clientID")]
    client_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SetupResponse {
    TimingEvent {
        #[serde(rename = "eventPort")]
        event_port: u16,
        #[serde(rename = "timingPort")]
        timing_port: u16,
        #[serde(rename = "timingPeerInfo")]
        timing_peer_info: Option<TimingPeerInfo>,
    },
    DataControl {
        streams: Vec<StreamResponse>,
    },
}

#[derive(Debug, Serialize)]
pub struct TimingPeerInfo {
    #[serde(rename = "Addresses")]
    addresses: Vec<String>,
    #[serde(rename = "ID")]
    id: String,
}

#[derive(Debug, Serialize)]
pub struct StreamResponse {
    #[serde(rename = "streamID")]
    stream_id: u32,
    #[serde(rename = "type")]
    ty: u8,
    #[serde(rename = "controlPort")]
    control_port: u16,
    #[serde(rename = "dataPort")]
    data_port: u16,
    #[serde(rename = "audioBufferSize")]
    auido_buffer_size: u32,
}

pub async fn handler(
    State(SharedState(state)): State<SharedState>,
    ConnectInfo(IncomingStream {
        local_addr,
        remote_addr,
        adv_data,
        ..
    }): ConnectInfo<IncomingStream>,
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> Result<BinaryPlist<SetupResponse>, StatusCode> {
    let bind_addr = local_addr.map_or_else(|| Ipv4Addr::new(0, 0, 0, 0).into(), |addr| addr.ip());
    match req {
        SetupRequest::InfoEvent {
            device_id,
            mac_addr,
            os_name,
            os_version,
            os_build_version,
            model,
            name,
            timing_protocol,
            ..
        } => {
            let (event_listener, event_port) = match open_tcp(bind_addr).await {
                Ok(res) => res,
                Err(err) => {
                    tracing::error!(%err, "failed to open event channel");
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            };

            let sender_info: SenderInfo = SenderInfo {
                device_id,
                name,
                model,
                os_name,
                os_version,
                os_build_version,

                mac_address: match mac_addr.parse() {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::error!(%err, "invalid mac address format");
                        return Err(StatusCode::BAD_REQUEST);
                    }
                },

                timing_proto: match timing_protocol.as_str() {
                    "PTP" | "ptp" => TimingProtocol::Ptp,
                    "NTP" | "ntp" => TimingProtocol::Ntp,
                    _ => {
                        tracing::error!(?timing_protocol, "invalid timing protocol");
                        return Err(StatusCode::BAD_REQUEST);
                    }
                },

                cancellation_token: CancellationToken::new(),
            };

            // TODO : timing?
            tokio::spawn(event_handler(
                event_listener,
                sender_info.cancellation_token.child_token(),
            ));

            let response = BinaryPlist(SetupResponse::TimingEvent {
                event_port,

                timing_port: match sender_info.timing_proto {
                    TimingProtocol::Ptp => 0,
                    TimingProtocol::Ntp => unimplemented!("NTP not supported"),
                },

                timing_peer_info: match sender_info.timing_proto {
                    TimingProtocol::Ptp => Some(TimingPeerInfo {
                        id: adv_data.mac_addr.to_string(),
                        addresses: vec![bind_addr.to_string()],
                    }),
                    TimingProtocol::Ntp => None,
                },
            });
            *state.sender_info.write().unwrap() = Some(sender_info);

            Ok(response)
        }

        SetupRequest::DataControl { streams } => {
            let Some(streams_token) = state
                .sender_info
                .read()
                .unwrap()
                .as_ref()
                .map(|s| s.cancellation_token.child_token())
            else {
                tracing::error!("uninitialized sender info");
                return Err(StatusCode::BAD_REQUEST);
            };

            let mut streams_out = Vec::with_capacity(streams.len());
            for stream_in in streams {
                // TODO : may be tcp or udp
                let (data_socket, data_port) = match open_tcp(bind_addr /*, None*/).await {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::error!(%err, "failed to open data channel");
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                };

                let (control_socket, control_port) = match open_udp(
                    bind_addr,
                    stream_in
                        .control_port
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

                let stream: Stream = Stream {
                    id: rand::random(),
                    client_id: stream_in.client_id,
                    cancellation_token: streams_token.child_token(),

                    ty: match stream_in.ty {
                        96 => StreamType::AudioRealTime,
                        103 => StreamType::AudioBuffered,
                        stream_type => {
                            tracing::error!(%stream_type, "unsupported stream type");
                            return Err(StatusCode::BAD_REQUEST);
                        }
                    },

                    metadata: StreamMetadata::Audio {
                        // TODO : what's that?
                        audio_buffer_size: stream_in.samples_per_frame,
                        latency_min: stream_in.latency_min,
                        latency_max: stream_in.latency_max,
                    },
                };

                tokio::spawn(data_handler(
                    data_socket,
                    stream.cancellation_token.child_token(),
                ));
                tokio::spawn(control_handler(
                    control_socket,
                    stream.cancellation_token.child_token(),
                ));

                streams_out.push(StreamResponse {
                    stream_id: stream.id,
                    ty: stream_in.ty,
                    control_port,
                    data_port,

                    auido_buffer_size: match stream.metadata {
                        StreamMetadata::Audio {
                            audio_buffer_size, ..
                        } => audio_buffer_size,
                    },
                });
                state.streams.write().unwrap().push(stream);
            }

            Ok(BinaryPlist(SetupResponse::DataControl {
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
async fn data_handler(listener: TcpListener, token: CancellationToken) {
    tokio::select! {
        _ = token.cancelled() => {
        }

        _ = future::pending() => {
        }
    }
}

async fn control_handler(socket: UdpSocket, token: CancellationToken) {
    tokio::select! {
        _ = token.cancelled() => {
        }

        _ = future::pending() => {
        }
    }
}

async fn event_handler(listener: TcpListener, token: CancellationToken) {
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                break;
            }

            _ = async {
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
            } => {}
        }
    }
}

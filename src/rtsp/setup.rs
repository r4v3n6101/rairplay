use std::net::{IpAddr, SocketAddr};
use std::{
    future, io,
    net::Ipv4Addr,
    sync::{Arc, Weak},
};

use axum::handler::HandlerWithoutStateExt;
use axum::{
    extract::{ConnectInfo, Path, State},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::net::{TcpStream, UdpSocket};
use tokio::{io::AsyncReadExt, net::TcpListener};
use tracing::{debug, error, info, trace, warn};

use crate::transport::IncomingStream;

use super::{
    plist::BinaryPlist,
    state::{Connection, Connections},
};

#[derive(Debug, Deserialize)]
pub struct StreamDesc {
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SetupRequest {
    InfoEvent {
        #[serde(rename = "timingProtocol")]
        timing_protocol: String,
        #[serde(rename = "ekey")]
        encryption_key: Bytes,
        #[serde(rename = "eiv")]
        encryption_iv: Bytes,
        #[serde(rename = "deviceID")]
        device_id: String,
        #[serde(rename = "macAddress")]
        mac_addr: String,
        #[serde(rename = "osName")]
        os_name: String,
        #[serde(rename = "osVersion")]
        os_version: String,
        model: String,
        name: String,
    },
    DataControl {
        streams: Vec<StreamDesc>,
    },
}

#[derive(Debug, Serialize)]
struct TimingPeerInfo {
    #[serde(rename = "Addresses")]
    addresses: Vec<String>,
    #[serde(rename = "ID")]
    id: String,
}

#[derive(Debug, Serialize)]
struct StreamOut {
    #[serde(rename = "type")]
    ty: u8,
    #[serde(rename = "controlPort")]
    control_port: u16,
    #[serde(rename = "dataPort")]
    data_port: u16,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum SetupResponse {
    TimingEvent {
        #[serde(rename = "eventPort")]
        event_port: u16,
        #[serde(rename = "timingPort")]
        timing_port: u16,
        #[serde(rename = "timingPeerInfo")]
        timing_peer_info: TimingPeerInfo,
    },
    DataControl {
        streams: Vec<StreamOut>,
    },
}

pub async fn handler(
    Path(media_id): Path<String>,
    State(Connections(connections)): State<Connections>,
    ConnectInfo(IncomingStream {
        local_addr,
        remote_addr,
        adv_data,
        ..
    }): ConnectInfo<IncomingStream>,
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> Response {
    let bind_addr = local_addr.map_or_else(|| Ipv4Addr::new(0, 0, 0, 0).into(), |addr| addr.ip());
    match req {
        SetupRequest::InfoEvent { .. } => {
            match TcpListener::bind((bind_addr, 0)).await {
                Ok(listener) => {
                    let event_port = match listener.local_addr() {
                        Ok(addr) => {
                            info!(%addr, "event listener opened");
                            addr.port()
                        }
                        Err(err) => {
                            error!(%err, "failed to get address of event listener");
                            return (StatusCode::INTERNAL_SERVER_ERROR, "unknown event port")
                                .into_response();
                        }
                    };

                    let connection = Arc::default();
                    if connections
                        .insert(media_id, Arc::clone(&connection))
                        .is_none()
                    {
                        info!("created new connection state");
                    } else {
                        warn!("replaced old connection state");
                    }

                    tokio::spawn(event_handler(listener, Arc::downgrade(&connection)));

                    // TODO : timingPort = 0 only for PTP
                    BinaryPlist(SetupResponse::TimingEvent {
                        event_port,
                        timing_port: 0,
                        timing_peer_info: TimingPeerInfo {
                            id: adv_data.mac_addr.to_string(),
                            addresses: vec![bind_addr.to_string()],
                        },
                    })
                    .into_response()
                }
                Err(err) => {
                    error!(%err, "failed to open event listener");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "event listener not opened",
                    )
                        .into_response()
                }
            }
        }
        SetupRequest::DataControl { streams } => {
            let mut streams_out = Vec::with_capacity(streams.len());
            for stream in &streams {
                let Some(connection) = connections.get(&media_id) else {
                    return (StatusCode::NOT_FOUND, "connection not found").into_response();
                };

                let (data_socket, data_port) = match open_udp(bind_addr, None).await {
                    Ok(res) => res,
                    Err(err) => {
                        error!(%err, "failed to open data channel");
                        return (StatusCode::INTERNAL_SERVER_ERROR, "data channel not opened")
                            .into_response();
                    }
                };
                let (control_socket, control_port) = match open_udp(
                    bind_addr,
                    stream
                        .control_port
                        .map(|port| SocketAddr::new(remote_addr.ip(), port)),
                )
                .await
                {
                    Ok(res) => res,
                    Err(err) => {
                        error!(%err, "failed to open control channel");
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "control channel not opened",
                        )
                            .into_response();
                    }
                };

                tokio::spawn(data_handler(data_socket, Arc::downgrade(&connection)));
                tokio::spawn(control_handler(control_socket, Arc::downgrade(&connection)));

                streams_out.push(StreamOut {
                    ty: stream.ty,
                    data_port,
                    control_port,
                });
            }

            BinaryPlist(SetupResponse::DataControl {
                streams: streams_out,
            })
            .into_response()
        }
    }
}

async fn open_udp(
    local_addr: IpAddr,
    remote_addr: Option<SocketAddr>,
) -> io::Result<(UdpSocket, u16)> {
    let socket = UdpSocket::bind(SocketAddr::new(local_addr, 0)).await?;
    if let Some(remote_addr) = remote_addr {
        socket.connect(remote_addr).await?;
    }
    let port = socket.local_addr()?.port();

    Ok((socket, port))
}

// TODO : this may be TCP
async fn data_handler(listener: UdpSocket, handle: Weak<Connection>) {
    future::pending().await
}

// TODO : this may be TCP
async fn control_handler(listener: UdpSocket, handle: Weak<Connection>) {
    future::pending().await
}

async fn event_handler(listener: TcpListener, handle: Weak<Connection>) {
    loop {
        let Some(connection) = handle.upgrade() else {
            info!("event listener closed");
            break;
        };

        match listener.accept().await {
            Ok((mut stream, remote_addr)) => {
                let mut buf = [0; 8 * 1024];
                while let Ok(len @ 1..) = stream.read(&mut buf).await {
                    debug!(%len, %remote_addr, "event stream bytes");
                }
            }
            Err(err) => {
                error!(%err, "event listener couldn't accept a connection");
            }
        }
    }
}

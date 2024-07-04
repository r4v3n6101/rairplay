use std::{
    future, io,
    net::{IpAddr, SocketAddr},
};

use axum::extract::ConnectInfo;
use futures::future::abortable;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, UdpSocket};

use crate::{
    service::{
        dto::{SenderInfo, StreamDescriptor, StreamInfo, TimingPeerInfo},
        plist::BinaryPlist,
    },
    transport::IncomingStream,
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
    ConnectInfo(IncomingStream {
        local_addr,
        remote_addr,
    }): ConnectInfo<IncomingStream>,
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> Result<BinaryPlist<SetupResponse>, StatusCode> {
    match req {
        SetupRequest::SenderInfo(info) => {
            tracing::info!(?info, "sender's info");
            Err(StatusCode::OK)
        }

        SetupRequest::Streams { streams } => {
            let mut streams_out = Vec::with_capacity(streams.len());
            for info in streams {
                // TODO : may be tcp or udp
                let (data_socket, local_data_port) = match open_tcp(local_addr /*, None*/).await {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::error!(%err, "failed to open data channel");
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                };
                let (data_task, data_handle) = abortable(tcp_tracing(data_socket));
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
                    audio_buffer_size: 44100 * 1024,

                    id: rand::random(),
                    ty: info.ty,

                    local_control_port,
                    local_data_port,
                };
                tokio::spawn(data_task);
                tokio::spawn(control_task);

                tracing::info!(?descriptor, ?info, "new stream");
                streams_out.push(descriptor);
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
async fn tcp_tracing(listener: TcpListener) {
    future::pending().await
}

async fn udp_tracing(socket: UdpSocket) {
    future::pending().await
}

async fn event_handler(listener: TcpListener) {
    future::pending().await
}

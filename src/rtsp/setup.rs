use std::{
    future, io,
    net::{IpAddr, SocketAddr},
};

use axum::extract::{ConnectInfo, State};
use futures::future::abortable;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, UdpSocket},
};

use crate::transport::IncomingStream;

use super::{
    dto::{SenderInfo, StreamDescriptor, StreamInfo, TimingPeerInfo},
    plist::BinaryPlist,
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
    State(SharedState {
        state, adv_data, ..
    }): State<SharedState>,
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
                    id: adv_data.mac_addr.to_string(),
                    addresses: vec![local_addr],
                }),
            });
            *state.sender.write().unwrap() = Some(sender);

            Ok(response)
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
                let (data_task, data_handle) = abortable(data_handler(data_socket));
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
                let (control_task, control_handle) = abortable(control_handler(control_socket));

                let descriptor = StreamDescriptor {
                    id: rand::random(),

                    ty: info.ty,
                    metadata: info.metadata.clone(),

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
async fn data_handler(listener: TcpListener) {
    future::pending().await
}

async fn control_handler(socket: UdpSocket) {
    future::pending().await
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

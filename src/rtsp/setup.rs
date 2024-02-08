use std::{
    net::Ipv4Addr,
    sync::{Arc, Weak},
};

use axum::{
    extract::{ConnectInfo, Path, State},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use hyper::StatusCode;
use plist::Value;
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncReadExt, net::TcpListener};
use tracing::{error, info, trace, warn};

use crate::transport::IncomingStream;

use super::{
    plist::BinaryPlist,
    state::{Connection, Connections},
};

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
        #[serde(flatten)]
        val: Value,
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
struct SetupResponse {
    #[serde(rename = "eventPort")]
    event_port: u16,
    #[serde(rename = "timingPort")]
    timing_port: u16,
    #[serde(rename = "timingPeerInfo")]
    timing_peer_info: TimingPeerInfo,
}

pub async fn handler(
    Path(media_id): Path<String>,
    State(Connections(connections)): State<Connections>,
    ConnectInfo(IncomingStream {
        local_addr,
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
                        .is_some()
                    {
                        warn!("replaced old connection");
                    } else {
                        info!("created new connection");
                    }

                    tokio::spawn(event_handler(listener, Arc::downgrade(&connection)));

                    // TODO : timingPort = 0 only for PTP
                    return BinaryPlist(SetupResponse {
                        event_port,
                        timing_port: 0,
                        timing_peer_info: TimingPeerInfo {
                            id: adv_data.mac_addr.to_string(),
                            addresses: vec![bind_addr.to_string()],
                        },
                    })
                    .into_response();
                }
                Err(err) => {
                    error!(%err, "failed to open event listener");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "event listener not opened",
                    )
                        .into_response();
                }
            }
        }
        SetupRequest::DataControl { val } => {
            println!("{val:?}");
        }
    }

    todo!()
}

async fn event_handler(event_listener: TcpListener, handle: Weak<Connection>) {
    loop {
        let Some(connection) = handle.upgrade() else {
            info!("event listener closed");
            break;
        };

        match event_listener.accept().await {
            Ok((mut stream, remote_addr)) => {
                info!(%remote_addr, "new event stream");

                let mut buf = [0; 8 * 1024];
                while let Ok(len) = stream.read(&mut buf).await {
                    trace!(%len, "event stream bytes");
                }
            }
            Err(err) => {
                error!(%err, "event listener couldn't accept an connection");
            }
        }
    }
}

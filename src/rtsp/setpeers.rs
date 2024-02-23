use std::net::IpAddr;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use hyper::StatusCode;
use tracing::{info, warn};

use super::{
    plist::BinaryPlist,
    state::{ClockSync, Connections},
};

pub async fn handler(
    Path(media_id): Path<String>,
    State(Connections(connections)): State<Connections>,
    BinaryPlist(addresses): BinaryPlist<Vec<String>>,
) -> impl IntoResponse {
    match connections.get(&media_id) {
        Some(connection) => {
            match &mut *connection.clock_sync.lock().unwrap() {
                ClockSync::PTP { peers } => {
                    peers.clear();
                    peers.extend(addresses.iter().filter_map(
                        |addr| match addr.parse::<IpAddr>() {
                            Ok(addr) => Some(addr),
                            Err(err) => {
                                warn!(%addr, %err, "invalid address format");
                                None
                            }
                        },
                    ));
                    info!(?peers, "updated PTP addresses");

                    ().into_response()
                }
                ClockSync::NTP { .. } => {
                    (StatusCode::NOT_ACCEPTABLE, "NTP couldn't set peers").into_response()
                }
            }
        }
        None => (StatusCode::NOT_FOUND, "connection not found").into_response(),
    }
}

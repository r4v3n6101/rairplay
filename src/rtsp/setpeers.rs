use std::net::IpAddr;

use axum::extract::{Path, State};
use hyper::StatusCode;
use tracing::{error, info, warn};

use super::{
    plist::BinaryPlist,
    state::{ClockSync, Connections},
};

pub async fn handler(
    Path(media_id): Path<String>,
    State(Connections(connections)): State<Connections>,
    BinaryPlist(addresses): BinaryPlist<Vec<String>>,
) -> Result<(), StatusCode> {
    let connection = connections.entry(media_id).or_default().downgrade();
    let mut clock_sync = connection.clock_sync.lock().unwrap();
    match &mut *clock_sync {
        ClockSync::PTP { peers } => {
            peers.clear();
            peers.extend(
                addresses
                    .iter()
                    .filter_map(|addr| match addr.parse::<IpAddr>() {
                        Ok(addr) => Some(addr),
                        Err(err) => {
                            warn!(%addr, %err, "invalid address format");
                            None
                        }
                    }),
            );
            info!(?peers, "updated PTP addresses");

            Ok(())
        }
        ClockSync::NTP { .. } => {
            error!("SETPEERS doesn't work for NTP");
            Err(StatusCode::NOT_ACCEPTABLE)
        }
    }
}

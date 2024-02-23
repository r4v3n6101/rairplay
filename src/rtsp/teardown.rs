use axum::extract::{Path, State};
use tracing::{info, warn};

use super::state::Connections;

pub async fn handler(
    Path(media_id): Path<String>,
    State(Connections(connections)): State<Connections>,
) {
    if connections.remove(&media_id).is_some() {
        info!("connection removed");
    } else {
        warn!("nothing to remove");
    }
}

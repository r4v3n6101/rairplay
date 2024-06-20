use axum::extract::State;
use serde::Deserialize;

use crate::{
    plist::BinaryPlist,
    rtsp::{dto::StreamType, state::SharedState},
};

#[derive(Debug, Deserialize)]
struct StreamId {
    #[serde(rename = "streamID")]
    id: u32,
    #[serde(rename = "type")]
    ty: StreamType,
}

#[derive(Debug, Deserialize)]
pub struct TeardownRequest {
    #[serde(rename = "streams")]
    streams_to_remove: Option<Vec<StreamId>>,
}

pub async fn handler(
    State(SharedState { state, .. }): State<SharedState>,
    BinaryPlist(TeardownRequest { streams_to_remove }): BinaryPlist<TeardownRequest>,
) {
    match streams_to_remove {
        Some(streams_to_remove) => {
            let mut streams = state.streams.write().unwrap();
            for StreamId { id, ty } in streams_to_remove {
                if let Some(pos) = streams
                    .iter()
                    .position(|handle| handle.descriptor.id == id && handle.descriptor.ty == ty)
                {
                    streams.remove(pos);
                    tracing::info!(
                        ?id,
                        ?ty,
                        "stream handle removed, remains: {}",
                        streams.len()
                    );
                } else {
                    tracing::warn!(
                        ?id,
                        ?ty,
                        "stream handle not found, remains: {}",
                        streams.len()
                    );
                }
            }
        }
        None => {
            state.streams.write().unwrap().clear();
            tracing::info!("all stream handles removed");
        }
    }
}

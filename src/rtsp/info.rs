use axum::{extract::State, response::IntoResponse};
use serde::Serialize;

use crate::plist::BinaryPlist;

use super::state::SharedState;

const PROTOVERS: &str = "1.1";
const SRCVERS: &str = "377.25.06";

#[derive(Debug, Serialize)]
struct MainResponse {
    #[serde(rename = "deviceid")]
    device_id: String,
    features: u64,
    manufacturer: String,
    model: String,
    name: String,
    #[serde(rename = "protovers")]
    protocol_version: String,
    source_version: String,
}

pub async fn handler(State(SharedState { adv, .. }): State<SharedState>) -> impl IntoResponse {
    let response = MainResponse {
        device_id: adv.mac_addr.to_string(),
        features: adv.features.bits(),
        protocol_version: PROTOVERS.to_string(),
        source_version: SRCVERS.to_string(),

        manufacturer: adv.manufacturer.clone(),
        model: adv.model.clone(),
        name: adv.name.clone(),
    };
    tracing::info!(?response, ?adv, "built info from advertisment");

    BinaryPlist(response)
}

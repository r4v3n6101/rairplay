use axum::{extract::ConnectInfo, response::IntoResponse};
use serde::Serialize;
use tracing::info;

use crate::transport::IncomingStream;

use super::plist::BinaryPlist;

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

pub async fn handler(
    ConnectInfo(IncomingStream { adv_data, .. }): ConnectInfo<IncomingStream>,
) -> impl IntoResponse {
    let response = MainResponse {
        device_id: adv_data.mac_addr.to_string(),
        features: adv_data.features.bits(),
        protocol_version: PROTOVERS.to_string(),
        source_version: SRCVERS.to_string(),

        manufacturer: adv_data.manufacturer.clone(),
        model: adv_data.model.clone(),
        name: adv_data.name.clone(),
    };
    info!(?response, ?adv_data, "built info from advertisment");

    BinaryPlist(response)
}

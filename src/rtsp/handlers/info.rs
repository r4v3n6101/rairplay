use axum::{extract::State, response::IntoResponse};
use serde::Serialize;

use crate::{plist::BinaryPlist, rtsp::state::SharedState};

const PROTOVERS: &str = "1.1";
const SRCVERS: &str = "377.25.06";

#[derive(Debug, Serialize)]
struct Display {
    #[serde(rename = "widthPixels")]
    width: u32,
    #[serde(rename = "heightPixels")]
    height: u32,
    uuid: String,
    #[serde(rename = "maxFPS")]
    max_fps: u32,
    features: u32,
}

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
    #[serde(rename = "keepAliveSendStatsAsBody")]
    keep_alive_send_stats_as_body: bool,
    displays: Vec<Display>,
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

        keep_alive_send_stats_as_body: true,
        displays: vec![Display {
            width: 1920,
            height: 1080,
            uuid: "0B7520E-11AD-4AEA-9D27-9E853690788F".to_string(),
            max_fps: 60,
            features: 2,
        }],
    };
    tracing::info!(?response, ?adv, "built info from advertisment");

    BinaryPlist(response)
}

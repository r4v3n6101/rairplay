use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::info::AppInfo;

use super::plist::BinaryPlist;

const PROTOVERS: &str = "1.1";
const SRCVERS: &str = "377.25.06";
const FEATURES: &str = "0x405fc200,0x8300";
const MAC_ADDR: &str = "9F:D7:AF:1F:D3:CD";

// TODO : keep just response behind the Arc
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub app_info: Arc<AppInfo>,
    pub initial_volume: f32,
}

#[derive(Debug, Deserialize)]
pub struct MainRequest {
    qualifier: Vec<String>,
}

#[derive(Debug, Serialize)]
struct MainResponse {
    #[serde(rename = "deviceid")]
    device_id: String,
    features: String,
    flags: String,
    manufacturer: String,
    model: String,
    name: String,
    #[serde(rename = "protovers")]
    protocol_version: String,
    #[serde(rename = "rsf")]
    required_sender_flags: String,
    #[serde(rename = "srcvers")]
    source_version: String,
}

#[derive(Debug, Serialize)]
struct AdditionalResponse {
    #[serde(rename = "initialVolume")]
    initial_volume: f32,
}

pub async fn handler(
    State(ServiceInfo {
        app_info,
        initial_volume,
    }): State<ServiceInfo>,
    req: Option<BinaryPlist<MainRequest>>,
) -> impl IntoResponse {
    if let Some(BinaryPlist(MainRequest { qualifier })) = req {
        info!(?qualifier, "requested main info with qualifier");
        (
            StatusCode::OK,
            BinaryPlist(MainResponse {
                device_id: MAC_ADDR.to_string(),
                flags: "0x4".to_string(),
                features: FEATURES.to_string(),
                required_sender_flags: "0x0".to_string(),
                protocol_version: PROTOVERS.to_string(),
                source_version: SRCVERS.to_string(),

                manufacturer: app_info.manufacturer.clone(),
                model: app_info.model.clone(),
                name: app_info.name.clone(),
            }),
        )
            .into_response()
    } else {
        info!(%initial_volume, "nothing's requested, initial volume responded");
        (
            StatusCode::OK,
            BinaryPlist(AdditionalResponse { initial_volume }),
        )
            .into_response()
    }
}

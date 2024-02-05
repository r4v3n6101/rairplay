use axum::{extract::Request, response::IntoResponse};
use hyper::{header::CONTENT_TYPE, StatusCode};
use serde::{Deserialize, Serialize};

use super::plist::BinaryPlist;

const SRCVERS: &str = "377.25.06";
const FEATURES: &str = "0x405fc200,0x8300";
const MAC_ADDR: &str = "9F:D7:AF:1F:D3:CD";

#[derive(Debug, Serialize, Deserialize)]
pub struct InfoRequest {
    pub qualifier: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    #[serde(rename = "deviceid")]
    pub device_id: String,
    pub features: String,
    pub flags: String,
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    #[serde(rename = "protovers")]
    pub protocol_version: String,
    #[serde(rename = "rsf")]
    pub required_sender_flags: String,
    #[serde(rename = "srcvers")]
    pub source_version: String,
}

#[derive(Debug, Serialize)]
pub struct AdditionalInfoResponse {
    #[serde(rename = "initialVolume")]
    initial_volume: f32,
}

impl Default for InfoResponse {
    fn default() -> Self {
        Self {
            device_id: MAC_ADDR.into(),
            features: FEATURES.into(),
            flags: "0x4".into(),
            manufacturer: env!("CARGO_PKG_AUTHORS").into(),
            model: env!("CARGO_PKG_NAME").into(),
            name: env!("CARGO_PKG_NAME").into(),
            protocol_version: "1.1".into(),
            required_sender_flags: "0x0".into(),
            source_version: SRCVERS.into(),
        }
    }
}

pub async fn handler(req: Request) -> impl IntoResponse {
    match req.headers().get(CONTENT_TYPE) {
        Some(_) => plist_handler(req).await.into_response(),
        None => empty_handler(req).await.into_response(),
    }
}

async fn plist_handler(req: Request) -> impl IntoResponse {
    (StatusCode::OK, BinaryPlist(InfoResponse::default()))
}

async fn empty_handler(req: Request) -> impl IntoResponse {
    (
        StatusCode::OK,
        BinaryPlist(AdditionalInfoResponse {
            initial_volume: -140.0,
        }),
    )
}

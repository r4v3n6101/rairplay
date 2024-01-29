use axum::response::IntoResponse;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use super::plist::BinaryPlist;

const GROUP_UUID: &str = "89581713-3fa2-4d2d-8a0e-b6840cf6b3ae";
const FEATURES: &str = "0x401FC200,0x300";
const MAC_ADDR: &str = "9F:D7:AF:1F:D3:CD";

#[derive(Debug, Serialize, Deserialize)]
pub struct InfoRequest {
    pub qualifier: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    #[serde(rename = "acl")]
    pub access_control_level: u8,
    #[serde(rename = "deviceid")]
    pub device_id: String,
    pub features: String,
    pub flags: String,
    #[serde(rename = "gcgl")]
    pub group_containing_discoverable_leader: u8,
    #[serde(rename = "gid")]
    pub group_uuid: String,
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    #[serde(rename = "protovers")]
    pub protocol_version: String,
    #[serde(rename = "rsf")]
    pub required_sender_flags: String,
    #[serde(rename = "serialNumber")]
    pub serial_number: String,
    #[serde(rename = "srcvers")]
    pub source_version: String,
    #[serde(rename = "pi")]
    pub pairing_uuid: String,
    #[serde(rename = "pk")]
    pub pubkey: String,
}

impl Default for InfoResponse {
    fn default() -> Self {
        Self {
            access_control_level: 0,
            device_id: MAC_ADDR.into(),
            features: FEATURES.into(),
            flags: "0x4".into(),
            group_containing_discoverable_leader: 0,
            group_uuid: GROUP_UUID.into(),
            manufacturer: env!("CARGO_PKG_AUTHORS").into(),
            model: env!("CARGO_PKG_NAME").into(),
            name: env!("CARGO_PKG_NAME").into(),
            protocol_version: "1.1".into(),
            required_sender_flags: "0x0".into(),
            serial_number: MAC_ADDR.into(),
            source_version: "366.0".into(),
            pairing_uuid: GROUP_UUID.into(),
            pubkey: Default::default(),
        }
    }
}

pub async fn handler(BinaryPlist(req): BinaryPlist<InfoRequest>) -> impl IntoResponse {
    (StatusCode::OK, BinaryPlist(InfoResponse::default()))
}

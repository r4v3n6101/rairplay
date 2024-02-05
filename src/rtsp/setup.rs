use std::{mem::ManuallyDrop, net::TcpListener};

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::plist::BinaryPlist;

#[derive(Debug, Deserialize)]
pub struct SetupRequest {
    #[serde(rename = "deviceID")]
    pub device_id: String,
    #[serde(rename = "eiv")]
    pub encryption_iv: Bytes,
    #[serde(rename = "ekey")]
    pub encryption_key: Bytes,
}

#[derive(Debug, Serialize)]
pub struct SetupResponse {
    #[serde(rename = "eventPort")]
    pub event_port: u16,
}

pub async fn handler(BinaryPlist(req): BinaryPlist<SetupRequest>) -> BinaryPlist<SetupResponse> {
    let listener = ManuallyDrop::new(TcpListener::bind("0.0.0.0:0").unwrap());
    let port = listener.local_addr().unwrap().port();

    BinaryPlist(SetupResponse { event_port: port })
}

use std::net::IpAddr;

use bytes::BytesMut;
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Deserialize)]
pub struct SenderInfo {
    #[serde(rename = "deviceID")]
    pub device_id: String,
    pub name: String,
    pub model: String,
    #[serde(rename = "macAddress")]
    pub mac_address: MacAddress,
    #[serde(rename = "osName")]
    pub os_name: Option<String>,
    #[serde(rename = "osVersion")]
    pub os_version: Option<String>,
    #[serde(rename = "osBuildVersion")]
    pub os_build_version: Option<String>,
    #[serde(rename = "timingProtocol")]
    pub timing_proto: TimingProtocol,
    #[serde(rename = "timingPeerInfo")]
    pub timing_info: Option<TimingPeerInfo>,
    #[serde(rename = "timingPeerList")]
    pub timing_peers: Option<Vec<TimingPeerInfo>>,
}

#[derive(Debug, Deserialize)]
pub enum TimingProtocol {
    #[serde(rename = "PTP")]
    Ptp,
    #[serde(rename = "NTP")]
    Ntp,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TimingPeerInfo {
    #[serde(rename = "Addresses")]
    pub addresses: Vec<IpAddr>,
    #[serde(rename = "ID")]
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct StreamInfo {
    #[serde(rename = "type")]
    pub ty: StreamType,
    #[serde(rename = "clientID")]
    pub client_id: Option<String>,
    #[serde(rename = "controlPort")]
    pub remote_control_port: Option<u16>,

    // BytesMut semantically equals to owning
    // So we can only move out key and no other access to it
    #[serde(rename = "shk")]
    pub shared_key: Option<BytesMut>,
    #[serde(rename = "shiv")]
    pub shared_iv: Option<BytesMut>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
#[non_exhaustive]
pub enum StreamType {
    AudioRealTime = 96,
    AudioBuffered = 103,
    // Screen (110)
    // Playback (120)
    // RemoteControl (130)
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamDescriptor {
    #[serde(rename = "streamID")]
    pub id: u32,
    #[serde(rename = "type")]
    pub ty: StreamType,
    #[serde(rename = "controlPort")]
    pub local_control_port: u16,
    #[serde(rename = "dataPort")]
    pub local_data_port: u16,
    // TODO : it may be unnecessary field for other stream types (like video)
    #[serde(rename = "audioBufferSize")]
    pub audio_buffer_size: u32,
}

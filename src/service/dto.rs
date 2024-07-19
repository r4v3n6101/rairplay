use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct InfoResponse {
    #[serde(rename = "deviceid")]
    pub device_id: String,
    #[serde(rename = "macAddress")]
    pub mac_addr: String,
    pub features: u64,
    pub manufacturer: String,
    pub model: String,
    pub name: String,

    // TODO : this naming is temporarily, I'll change it if not working
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(rename = "sourceVersion")]
    pub source_version: String,
    #[serde(rename = "initialVolume")]
    pub initial_volume: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SetupRequest {
    SenderInfo {
        #[serde(flatten)]
        content: plist::Value,
    },
    Streams {
        streams: Vec<StreamInfo>,
    },
}

#[derive(Debug, Deserialize)]
pub struct StreamInfo {
    // TODO : by types
    #[serde(rename = "type")]
    pub ty: (),
    // TODO : for audio only!
    pub remote_control_port: Option<u16>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SetupResponse {
    // TODO : is this actually needed?
    Initial {
        #[serde(rename = "eventPort")]
        event_port: u16,
        #[serde(rename = "timingPort")]
        timing_port: u16,
        #[serde(rename = "timingPeerInfo")]
        timing_peer_info: Option<()>,
    },
    Streams {
        streams: Vec<StreamDescriptor>,
    },
}

// TODO : same as above, this is enum
#[derive(Debug, Serialize)]
pub struct StreamDescriptor {
    #[serde(rename = "streamID")]
    pub id: u32,
    #[serde(rename = "dataPort")]
    pub local_data_port: u16,
    #[serde(rename = "controlPort")]
    pub local_control_port: Option<u16>,
    #[serde(rename = "type")]
    pub ty: (),

    // TODO : seems specific only for audio
    #[serde(rename = "audioBufferSize")]
    pub audio_buffer_size: u32,
}

#[derive(Debug, Deserialize)]
pub struct FlushBufferedRequest {
    #[serde(rename = "flushUntilSeq")]
    flush_until_seqnum: Option<u32>,
    #[serde(rename = "flushUntilTS")]
    flush_until_timestamp: Option<u32>,
    #[serde(rename = "flushFromSeq")]
    flush_from_seqnum: Option<u32>,
    #[serde(rename = "flushFromTS")]
    flush_from_timestamp: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SetRateAnchorTimeRequest {
    rate: f32,
    #[serde(rename = "rtpTime")]
    anchor_rtp_timestamp: Option<u64>,
}

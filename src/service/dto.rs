use bytes::Bytes;
use mac_address::MacAddress;
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

    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(rename = "sourceVersion")]
    pub source_version: String,
    #[serde(rename = "initialVolume")]
    pub initial_volume: Option<f32>,

    pub displays: Vec<Display>,
}

#[derive(Debug, Serialize)]
pub struct Display {
    #[serde(rename = "widthPixels")]
    pub width: u32,
    #[serde(rename = "heightPixels")]
    pub height: u32,
    pub uuid: String,
    #[serde(rename = "maxFPS")]
    pub max_fps: u32,
    pub features: u32,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SetupRequest {
    SenderInfo {
        name: String,
        model: String,
        #[serde(rename = "deviceID")]
        device_id: String,
        #[serde(rename = "macAddress")]
        mac_addr: MacAddress,

        #[serde(rename = "osName")]
        os_name: Option<String>,
        #[serde(rename = "osVersion")]
        os_version: Option<String>,
        #[serde(rename = "osBuildVersion")]
        os_build_version: Option<String>,

        #[serde(flatten)]
        timing_proto: TimingProtocol,

        #[serde(flatten)]
        content: plist::Value,
    },
    Streams {
        streams: Vec<StreamInfo>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "timingProtocol")]
pub enum TimingProtocol {
    #[serde(rename = "PTP")]
    Ptp {
        //#[serde(flatten, rename = "timingPeerInfo")]
        //peer_info: (),
        //#[serde(rename = "timingPeerList")]
        //peer_list: (),
    },
    #[serde(rename = "NTP")]
    Ntp {
        #[serde(rename = "timingPort")]
        remote_port: u16,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StreamInfo {
    #[serde(rename = 96)]
    AudioRealtime {
        #[serde(rename = "ct")]
        content_type: u8,
        #[serde(rename = "audioFormat")]
        audio_format: Option<u32>,
        #[serde(rename = "spf")]
        samples_per_frame: u32,
        #[serde(rename = "controlPort")]
        remote_control_port: u16,

        #[serde(flatten)]
        content: plist::Value,
    },
    #[serde(rename = 103)]
    AudioBuffered {
        #[serde(rename = "ct")]
        content_type: Option<u8>,
        #[serde(rename = "audioFormat")]
        audio_format: Option<u32>,
        #[serde(rename = "spf")]
        samples_per_frame: Option<u32>,
        #[serde(rename = "shk")]
        shared_key: Option<Bytes>,

        #[serde(flatten)]
        content: plist::Value,
    },
    #[serde(rename = 110)]
    Video {
        #[serde(flatten)]
        content: plist::Value,
    },
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

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum StreamDescriptor {
    #[serde(rename = 96)]
    AudioRealtime {
        #[serde(rename = "streamID")]
        id: u32,
        #[serde(rename = "dataPort")]
        local_data_port: u16,
        #[serde(rename = "controlPort")]
        local_control_port: Option<u16>,
        #[serde(rename = "audioBufferSize")]
        audio_buffer_size: u32,
    },
    #[serde(rename = 103)]
    AudioBuffered {
        #[serde(rename = "streamID")]
        id: u32,
        #[serde(rename = "dataPort")]
        local_data_port: u16,
        #[serde(rename = "audioBufferSize")]
        audio_buffer_size: u32,
    },
    #[serde(rename = 110)]
    Video {
        #[serde(rename = "streamID")]
        id: u32,
        #[serde(rename = "dataPort")]
        local_data_port: u16,
    },
}

#[derive(Debug, Deserialize)]
pub struct FlushBufferedRequest {
    #[serde(rename = "flushUntilSeq")]
    pub flush_until_seqnum: Option<u32>,
    #[serde(rename = "flushUntilTS")]
    pub flush_until_timestamp: Option<u32>,
    #[serde(rename = "flushFromSeq")]
    pub flush_from_seqnum: Option<u32>,
    #[serde(rename = "flushFromTS")]
    pub flush_from_timestamp: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SetRateAnchorTimeRequest {
    pub rate: f32,
    #[serde(rename = "rtpTime")]
    pub anchor_rtp_timestamp: Option<u64>,
}

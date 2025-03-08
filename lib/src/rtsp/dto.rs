use bytes::Bytes;
use macaddr::MacAddr6;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub enum StreamId {
    AudioRealtime = 96,
    AudioBuffered = 103,
    Video = 110,
}

#[derive(Serialize)]
pub struct InfoResponse {
    #[serde(rename = "deviceid")]
    pub device_id: MacAddr6,
    #[serde(rename = "macAddress")]
    pub mac_addr: MacAddr6,
    pub features: u64,
    pub manufacturer: String,
    pub model: String,
    pub name: String,

    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(rename = "sourceVersion")]
    pub source_version: String,

    pub displays: Vec<Display>,
}

#[derive(Serialize)]
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

#[derive(Deserialize)]
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

#[derive(Deserialize)]
pub struct SetRateAnchorTimeRequest {
    pub rate: f32,
    #[serde(rename = "rtpTime")]
    pub anchor_rtp_timestamp: Option<u64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum SetupRequest {
    SenderInfo {
        name: String,
        model: String,
        #[serde(rename = "deviceID")]
        device_id: String,
        #[serde(rename = "macAddress")]
        mac_addr: String,

        #[serde(rename = "osName")]
        os_name: Option<String>,
        #[serde(rename = "osVersion")]
        os_version: Option<String>,
        #[serde(rename = "osBuildVersion")]
        os_build_version: Option<String>,

        #[serde(flatten)]
        timing_proto: TimingProtocol,

        #[serde(rename = "ekey")]
        ekey: Bytes,
        #[serde(rename = "eiv")]
        eiv: Bytes,

        #[serde(flatten)]
        content: plist::Value,
    },
    Streams {
        #[serde(rename = "streams")]
        requests: Vec<StreamRequest>,
    },
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum StreamRequest {
    #[serde(rename = 96)]
    AudioRealtime {
        #[serde(rename = "ct")]
        content_type: u8,
        #[serde(rename = "audioFormat")]
        audio_format: u32,
        #[serde(rename = "spf")]
        samples_per_frame: u32,
        #[serde(rename = "sr")]
        sample_rate: u32,
        #[serde(rename = "latencyMin")]
        latency_min: u32,
        #[serde(rename = "latencyMax")]
        latency_max: u32,
        #[serde(rename = "shk")]
        shared_key: Option<Bytes>,
        #[serde(rename = "shiv")]
        shared_iv: Option<Bytes>,
        #[serde(rename = "controlPort")]
        remote_control_port: u16,
    },
    #[serde(rename = 103)]
    AudioBuffered {
        #[serde(rename = "ct")]
        content_type: Option<u8>,
        #[serde(rename = "audioFormat")]
        audio_format: u32,
        #[serde(rename = "audioFormatIndex")]
        audio_format_index: Option<u8>,
        #[serde(rename = "spf")]
        samples_per_frame: u32,
        #[serde(rename = "shk")]
        shared_key: Option<Bytes>,
        #[serde(rename = "shiv")]
        shared_iv: Option<Bytes>,
        #[serde(rename = "clientID")]
        client_id: Option<String>,
    },
    #[serde(rename = 110)]
    Video {
        #[serde(rename = "streamConnectionID")]
        stream_connection_id: i64,
        #[serde(rename = "latencyMs")]
        latency_ms: u32,
    },
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum SetupResponse {
    General {
        #[serde(rename = "eventPort")]
        event_port: u16,

        // TODO : this may be moved to NTP branch, because it's always zero for PTP and PTP
        // sometimes requires (or not) timingPeerInfo
        #[serde(rename = "timingPort")]
        timing_port: u16,
    },
    Streams {
        #[serde(rename = "streams")]
        response: Vec<StreamResponse>,
    },
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum StreamResponse {
    #[serde(rename = 96)]
    AudioRealtime {
        #[serde(rename = "streamID")]
        id: u64,
        #[serde(rename = "dataPort")]
        local_data_port: u16,
        #[serde(rename = "controlPort")]
        local_control_port: u16,
    },
    #[serde(rename = 103)]
    AudioBuffered {
        #[serde(rename = "streamID")]
        id: u64,
        #[serde(rename = "dataPort")]
        local_data_port: u16,
        #[serde(rename = "audioBufferSize")]
        audio_buffer_size: u32,
    },
    #[serde(rename = 110)]
    Video {
        #[serde(rename = "streamID")]
        id: u64,
        #[serde(rename = "dataPort")]
        local_data_port: u16,
    },
}

#[derive(Deserialize)]
pub struct Teardown {
    #[serde(rename = "streams")]
    pub requests: Option<Vec<TeardownRequest>>,
}

#[derive(Deserialize)]
pub struct TeardownRequest {
    #[serde(rename = "streamID")]
    pub id: Option<u64>,
    #[serde(rename = "type")]
    pub ty: u32,
}

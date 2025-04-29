use bytes::Bytes;
use macaddr::MacAddr6;
use serde::{Deserialize, Serialize};

pub struct StreamId;

impl StreamId {
    pub const AUDIO_REALTIME: u32 = 96;
    pub const AUDIO_BUFFERED: u32 = 103;
    pub const VIDEO: u32 = 110;
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
#[serde(untagged)]
pub enum SetupRequest {
    SenderInfo {
        #[serde(flatten)]
        info: SenderInfo,
    },
    Streams {
        #[serde(rename = "streams")]
        requests: Vec<StreamRequest>,
    },
}

#[derive(Deserialize)]
pub struct SenderInfo {
    pub name: String,
    pub model: String,
    #[serde(rename = "deviceID")]
    pub device_id: String,
    #[serde(rename = "macAddress")]
    pub mac_addr: String,
    #[serde(rename = "osName")]
    pub os_name: Option<String>,
    #[serde(rename = "osVersion")]
    pub os_version: Option<String>,
    #[serde(rename = "osBuildVersion")]
    pub os_build_version: Option<String>,
    #[serde(rename = "ekey")]
    pub ekey: Bytes,
    #[serde(rename = "eiv")]
    pub eiv: Bytes,

    #[serde(flatten)]
    pub timing_proto: TimingProtocol,

    #[serde(flatten)]
    pub content: plist::Value,
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
        #[serde(flatten)]
        request: AudioRealtimeRequest,
    },
    #[serde(rename = 103)]
    AudioBuffered {
        #[serde(flatten)]
        request: AudioBufferedRequest,
    },
    #[serde(rename = 110)]
    Video {
        #[serde(flatten)]
        request: VideoRequest,
    },
}

#[derive(Deserialize)]
pub struct AudioRealtimeRequest {
    #[serde(rename = "ct")]
    pub content_type: u8,
    #[serde(rename = "audioFormat")]
    pub audio_format: u32,
    #[serde(rename = "spf")]
    pub samples_per_frame: u32,
    #[serde(rename = "sr")]
    pub sample_rate: u32,
    #[serde(rename = "latencyMin")]
    pub min_latency_samples: u32,
    #[serde(rename = "latencyMax")]
    pub max_latency_samples: u32,
    #[serde(rename = "shk")]
    pub shared_key: Option<Bytes>,
    #[serde(rename = "shiv")]
    pub shared_iv: Option<Bytes>,
    #[serde(rename = "controlPort")]
    pub remote_control_port: u16,
}

#[derive(Deserialize)]
pub struct AudioBufferedRequest {
    #[serde(rename = "ct")]
    pub content_type: u8,
    #[serde(rename = "audioFormat")]
    pub audio_format: u32,
    #[serde(rename = "audioFormatIndex")]
    pub audio_format_index: Option<u8>,
    #[serde(rename = "spf")]
    pub samples_per_frame: u32,
    #[serde(rename = "shk")]
    pub shared_key: Bytes,
    #[serde(rename = "shiv")]
    pub shared_iv: Option<Bytes>,
    #[serde(rename = "clientID")]
    pub client_id: Option<String>,
}

#[derive(Deserialize)]
pub struct VideoRequest {
    #[serde(rename = "streamConnectionID")]
    pub stream_connection_id: i64,
    #[serde(rename = "latencyMs")]
    pub latency_ms: u32,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum SetupResponse {
    Info {
        #[serde(rename = "eventPort")]
        event_port: u16,

        // TODO : this may be moved to NTP branch, because it's always zero for PTP and PTP
        // sometimes requires (or not) timingPeerInfo
        #[serde(rename = "timingPort")]
        timing_port: u16,
    },
    Streams {
        #[serde(rename = "streams")]
        responses: Vec<StreamResponse>,
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

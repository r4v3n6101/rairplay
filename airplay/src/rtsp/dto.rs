#![allow(unused_variables, dead_code)]

use bytes::Bytes;
use macaddr::MacAddr6;
use plist::{from_value, Value};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
pub enum StreamType {
    AudioRealtime = 96,
    AudioBuffered = 103,
    Video = 110,
}

#[derive(Debug, Serialize)]
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
    SenderInfo(Box<SenderInfo>),
    Streams {
        #[serde(rename = "streams")]
        requests: Vec<StreamRequest>,
    },
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug)]
pub enum StreamRequest {
    AudioRealtime(AudioRealtimeRequest),
    AudioBuffered(AudioBufferedRequest),
    Video(VideoRequest),
}

#[derive(Debug, Deserialize)]
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
    #[serde(rename = "controlPort")]
    pub remote_control_port: u16,
}

#[derive(Debug, Deserialize)]
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
    #[serde(rename = "clientID")]
    pub client_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VideoRequest {
    #[serde(rename = "streamConnectionID")]
    pub stream_connection_id: i64,
    #[serde(rename = "latencyMs")]
    pub latency_ms: u32,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug)]
pub enum StreamResponse {
    AudioRealtime {
        id: u64,
        local_data_port: u16,
        local_control_port: u16,
    },
    AudioBuffered {
        id: u64,
        local_data_port: u16,
        audio_buffer_size: u32,
    },
    Video {
        id: u64,
        local_data_port: u16,
    },
}

#[derive(Debug, Deserialize)]
pub struct Teardown {
    #[serde(rename = "streams")]
    pub requests: Option<Vec<TeardownRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct TeardownRequest {
    #[serde(rename = "streamID")]
    pub id: Option<u64>,
    #[serde(rename = "type", deserialize_with = "deserialize_stream_type")]
    pub ty: StreamType,
}

fn deserialize_stream_type<'de, D>(deserializer: D) -> Result<StreamType, D::Error>
where
    D: Deserializer<'de>,
{
    let ty = u32::deserialize(deserializer)?;
    match ty {
        x if x == StreamType::AudioRealtime as u32 => Ok(StreamType::AudioRealtime),
        x if x == StreamType::AudioBuffered as u32 => Ok(StreamType::AudioBuffered),
        x if x == StreamType::Video as u32 => Ok(StreamType::Video),
        _ => Err(de::Error::custom("unknown stream type")),
    }
}

fn serialize_stream_type<S>(tag: &StreamType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u32(*tag as u32)
}

impl<'de> Deserialize<'de> for StreamRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        struct StreamRequestEnvelope {
            #[serde(rename = "type", deserialize_with = "deserialize_stream_type")]
            ty: StreamType,
            #[serde(flatten)]
            payload: Value,
        }

        let env = StreamRequestEnvelope::deserialize(deserializer)?;
        match env.ty {
            StreamType::AudioRealtime => Ok(StreamRequest::AudioRealtime(
                from_value(&env.payload).map_err(de::Error::custom)?,
            )),
            StreamType::AudioBuffered => Ok(StreamRequest::AudioBuffered(
                from_value(&env.payload).map_err(de::Error::custom)?,
            )),
            StreamType::Video => Ok(StreamRequest::Video(
                from_value(&env.payload).map_err(de::Error::custom)?,
            )),
        }
    }
}

impl Serialize for StreamResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Debug, Serialize)]
        struct StreamResponseAudioRealtime {
            #[serde(rename = "type", serialize_with = "serialize_stream_type")]
            ty: StreamType,
            #[serde(rename = "streamID")]
            id: u64,
            #[serde(rename = "dataPort")]
            local_data_port: u16,
            #[serde(rename = "controlPort")]
            local_control_port: u16,
        }

        #[derive(Debug, Serialize)]
        struct StreamResponseAudioBuffered {
            #[serde(rename = "type", serialize_with = "serialize_stream_type")]
            ty: StreamType,
            #[serde(rename = "streamID")]
            id: u64,
            #[serde(rename = "dataPort")]
            local_data_port: u16,
            #[serde(rename = "audioBufferSize")]
            audio_buffer_size: u32,
        }

        #[derive(Debug, Serialize)]
        struct StreamResponseVideo {
            #[serde(rename = "type", serialize_with = "serialize_stream_type")]
            ty: StreamType,
            #[serde(rename = "streamID")]
            id: u64,
            #[serde(rename = "dataPort")]
            local_data_port: u16,
        }

        match self {
            StreamResponse::AudioRealtime {
                id,
                local_data_port,
                local_control_port,
            } => StreamResponseAudioRealtime {
                ty: StreamType::AudioRealtime,
                id: *id,
                local_data_port: *local_data_port,
                local_control_port: *local_control_port,
            }
            .serialize(serializer),
            StreamResponse::AudioBuffered {
                id,
                local_data_port,
                audio_buffer_size,
            } => StreamResponseAudioBuffered {
                ty: StreamType::AudioBuffered,
                id: *id,
                local_data_port: *local_data_port,
                audio_buffer_size: *audio_buffer_size,
            }
            .serialize(serializer),
            StreamResponse::Video {
                id,
                local_data_port,
            } => StreamResponseVideo {
                ty: StreamType::Video,
                id: *id,
                local_data_port: *local_data_port,
            }
            .serialize(serializer),
        }
    }
}

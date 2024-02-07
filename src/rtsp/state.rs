use std::{net::IpAddr, sync::Arc};

use dashmap::DashMap;

#[derive(Debug)]
pub enum ClockSync {
    PTP { peers: Vec<IpAddr> },
    NTP { port: u16 },
}

#[derive(Debug)]
pub struct Audio {
    pub sample_rate: u32,
    pub sample_size: u32,
    pub samples_per_frame: u32,
    pub channels: u8,
    // TODO : audio codec, latencies, shk
}

#[derive(Debug)]
pub struct ConnectionState {
    // Connection initialized when SETUP with clock proto are performed
    pub clock_sync: ClockSync,
    // Only 1 stream for every connection, but may be changed lately (?)
    pub audio: Option<Audio>,

    pub volume: f32,
    pub playback_rate: f32,
    pub progress: (u32, u32, u32),
}

#[derive(Debug, Clone)]
pub struct Connections {
    pub connections: Arc<DashMap<String, ConnectionState>>,
}

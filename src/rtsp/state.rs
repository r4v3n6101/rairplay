use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use crossbeam_utils::atomic::AtomicCell;
use dashmap::DashMap;

#[derive(Debug)]
pub enum ClockSync {
    PTP { peers: Vec<IpAddr> },
    NTP { port: u16 },
}

impl Default for ClockSync {
    fn default() -> Self {
        ClockSync::PTP { peers: Vec::new() }
    }
}

#[derive(Debug)]
pub struct Audio {
    pub sample_rate: u32,
    pub sample_size: u32,
    pub samples_per_frame: u32,
    pub channels: u8,
    // TODO : audio codec, latencies, shk
}

#[derive(Debug, Default)]
pub struct Connection {
    pub clock_sync: Mutex<ClockSync>,
    // Only 1 stream for every connection, but may be changed lately (?)
    pub audio: Mutex<Option<Audio>>,

    pub volume: AtomicCell<f32>,
    pub playback_rate: AtomicCell<f32>,
    pub progress: AtomicCell<(u32, u32, u32)>,
}

#[derive(Debug, Default, Clone)]
pub struct Connections(pub Arc<DashMap<String, Arc<Connection>>>);

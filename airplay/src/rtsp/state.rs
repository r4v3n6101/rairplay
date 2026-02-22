use std::sync::{Arc, Mutex, Weak, atomic::AtomicU64};

use bytes::Bytes;
use seqlock::SeqLock;
use tokio::sync::Mutex as AsyncMutex;
use weak_table::WeakValueHashMap;

use crate::{
    config::Config,
    crypto::{AesIv128, AesKey128},
    playback::ChannelHandle,
    streaming::{EventChannel, SharedData},
};

pub struct ServiceState<ADev, VDev, KC> {
    pub last_stream_id: AtomicU64,
    pub fp_last_msg: Mutex<Bytes>,
    pub ekey: SeqLock<Option<AesKey128>>,
    pub eiv: SeqLock<Option<AesIv128>>,
    pub event_channel: AsyncMutex<Option<EventChannel>>,
    pub stream_channels: Mutex<WeakValueHashMap<(u64, u32), Weak<SharedData>>>,

    pub config: Arc<Config<ADev, VDev, KC>>,
}

impl<A, V, K> ServiceState<A, V, K> {
    pub fn new(config: Arc<Config<A, V, K>>) -> Self {
        Self {
            last_stream_id: AtomicU64::default(),
            fp_last_msg: Mutex::default(),
            ekey: SeqLock::default(),
            eiv: SeqLock::default(),
            event_channel: AsyncMutex::default(),
            stream_channels: Mutex::default(),

            config,
        }
    }
}

impl<A, V, K> Drop for ServiceState<A, V, K> {
    fn drop(&mut self) {
        // Just in case if the service is dropped, but channels still remain
        self.stream_channels
            .lock()
            .unwrap()
            .drain()
            .for_each(|(_, chan)| chan.close());
    }
}

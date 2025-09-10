use std::sync::{Arc, Mutex, Weak, atomic::AtomicU64};

use bytes::Bytes;
use tokio::sync::Mutex as AsyncMutex;
use weak_table::WeakValueHashMap;

use crate::{
    config::Config,
    crypto::{AesIv128, AesKey128},
    pairing::SessionKeyHolder,
    playback::ChannelHandle,
    streaming::EventChannel,
};

pub struct ServiceState<ADev, VDev> {
    pub last_stream_id: AtomicU64,
    pub fp_last_msg: Mutex<Bytes>,
    pub session_key: Mutex<Option<Bytes>>,
    pub ekey: Mutex<AesKey128>,
    pub eiv: Mutex<AesIv128>,
    pub event_channel: AsyncMutex<Option<EventChannel>>,
    pub stream_channels: Mutex<WeakValueHashMap<(u64, u32), Weak<dyn ChannelHandle>>>,

    pub config: Arc<Config<ADev, VDev>>,
}

impl<A, V> ServiceState<A, V> {
    pub fn new(config: Arc<Config<A, V>>) -> Self {
        Self {
            last_stream_id: AtomicU64::default(),
            fp_last_msg: Mutex::default(),
            session_key: Mutex::default(),
            ekey: Mutex::default(),
            eiv: Mutex::default(),
            event_channel: AsyncMutex::default(),
            stream_channels: Mutex::default(),

            config,
        }
    }
}

impl<A, V> SessionKeyHolder for ServiceState<A, V>
where
    A: Send + Sync,
    V: Send + Sync,
{
    fn set_session_key(&self, key: Bytes) {
        if !key.is_empty() {
            let _ = self.session_key.lock().unwrap().insert(key);
        }
    }
}

impl<A, V> Drop for ServiceState<A, V> {
    fn drop(&mut self) {
        // Just in case if the service is dropped, but channels still remain
        self.stream_channels
            .lock()
            .unwrap()
            .drain()
            .for_each(|(_, chan)| chan.close());
    }
}

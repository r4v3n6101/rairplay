use std::sync::{Arc, Mutex, Weak, atomic::AtomicU64};

use bytes::Bytes;
use tokio::sync::Mutex as AsyncMutex;
use weak_table::WeakValueHashMap;

use crate::{
    config::{Config, Keychain},
    crypto::{AesIv128, AesKey128},
    pairing::{KeychainHolder, SessionKeyHolder},
    playback::ChannelHandle,
    streaming::{EventChannel, SharedData},
};

pub struct ServiceState<ADev, VDev, Id> {
    pub last_stream_id: AtomicU64,
    pub fp_last_msg: Mutex<Bytes>,
    pub session_key: Mutex<Option<Bytes>>,
    pub ekey: Mutex<AesKey128>,
    pub eiv: Mutex<AesIv128>,
    pub event_channel: AsyncMutex<Option<EventChannel>>,
    pub stream_channels: Mutex<WeakValueHashMap<(u64, u32), Weak<SharedData>>>,

    pub config: Arc<Config<ADev, VDev, Id>>,
}

impl<A, V, I> ServiceState<A, V, I> {
    pub fn new(config: Arc<Config<A, V, I>>) -> Self {
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

impl<A, V, I> SessionKeyHolder for ServiceState<A, V, I>
where
    A: Send + Sync,
    V: Send + Sync,
    I: Send + Sync,
{
    fn set_session_key(&self, key: Bytes) {
        if !key.is_empty() {
            let _ = self.session_key.lock().unwrap().insert(key);
        }
    }
}

impl<A, V, K> KeychainHolder for ServiceState<A, V, K>
where
    A: Send + Sync,
    V: Send + Sync,
    K: Send + Sync,
    K: Keychain,
{
    type Keychain = K;

    fn keychain(&self) -> &Self::Keychain {
        &self.config.keychain
    }
}

impl<A, V, I> Drop for ServiceState<A, V, I> {
    fn drop(&mut self) {
        // Just in case if the service is dropped, but channels still remain
        self.stream_channels
            .lock()
            .unwrap()
            .drain()
            .for_each(|(_, chan)| chan.close());
    }
}

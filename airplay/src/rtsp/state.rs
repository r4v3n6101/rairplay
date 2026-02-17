use std::sync::{Arc, Mutex, Weak, atomic::AtomicU64};

use bytes::Bytes;
use seqlock::SeqLock;
use tokio::sync::Mutex as AsyncMutex;
use weak_table::WeakValueHashMap;

use crate::{
    config::{Config, Keychain},
    crypto::{AesIv128, AesKey128},
    pairing::{KeychainHolder, SessionKeyHolder},
    playback::ChannelHandle,
    streaming::{EventChannel, SharedData},
};

pub struct ServiceState<ADev, VDev, KC> {
    pub last_stream_id: AtomicU64,
    pub fp_last_msg: Mutex<Bytes>,
    pub session_key: Mutex<Option<Bytes>>,
    pub ekey: SeqLock<AesKey128>,
    pub eiv: SeqLock<AesIv128>,
    pub event_channel: AsyncMutex<Option<EventChannel>>,
    pub stream_channels: Mutex<WeakValueHashMap<(u64, u32), Weak<SharedData>>>,

    pub config: Arc<Config<ADev, VDev, KC>>,
}

impl<A, V, K> ServiceState<A, V, K> {
    pub fn new(config: Arc<Config<A, V, K>>) -> Self {
        Self {
            last_stream_id: AtomicU64::default(),
            fp_last_msg: Mutex::default(),
            session_key: Mutex::default(),
            ekey: SeqLock::default(),
            eiv: SeqLock::default(),
            event_channel: AsyncMutex::default(),
            stream_channels: Mutex::default(),

            config,
        }
    }
}

impl<A, V, K> SessionKeyHolder for ServiceState<A, V, K>
where
    A: Send + Sync,
    V: Send + Sync,
    K: Send + Sync,
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

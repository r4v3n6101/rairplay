use std::sync::Arc;

use bytes::Bytes;
use dashmap::DashMap;
use symphonia_core::codecs::CodecParameters;
use tracing::{info, warn};

use crate::crypto::KeyEncryption;

#[derive(Debug)]
pub struct Metadata {
    pub begin_pos: u32,
    pub current_pos: u32,
    pub end_pos: u32,
    pub volume: f64,
    pub artwork: Bytes,
}

#[derive(Debug)]
pub enum State {
    Announced {
        codec_params: CodecParameters,
        encryption: KeyEncryption,
    },
    Initiailized {
        metadata: Metadata,
    },
}

#[derive(Debug, Default, Clone)]
pub struct StateStorage {
    inner: Arc<DashMap<String, State>>,
}

impl StateStorage {
    pub fn insert(&self, id: String, state: State) {
        info!(%id, "new state created");
        if self.inner.insert(id, state).is_some() {
            warn!("replaced previous state");
        }
    }

    pub fn remove(&self, id: &str) {
        if self.inner.remove(id).is_some() {
            info!(%id, "removed state");
        } else {
            warn!(%id, "no state to remove");
        }
    }

    // TODO : result
    pub fn update_metadata(&self, id: &str, f: impl FnOnce(&mut Metadata)) {
        if let Some(State::Initiailized { metadata }) = self.inner.get_mut(id).as_deref_mut() {
            f(metadata);
        } else {
            warn!(%id, "no initialized state, skipping update");
        }
    }

    pub fn fetch_metadata<T>(&self, id: &str, f: impl FnOnce(&Metadata) -> T) -> Option<T> {
        if let Some(State::Initiailized { metadata }) = self.inner.get(id).as_deref() {
            Some(f(metadata))
        } else {
            None
        }
    }
}

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
        if let Some(_) = self.inner.insert(id, state) {
            warn!("replaced previous state");
        }
    }

    pub fn remove(&self, id: &str) {
        if let Some(_) = self.inner.remove(id) {
            info!(%id, "removed state");
        } else {
            warn!(%id, "no state to remove");
        }
    }

    pub fn has(&self, id: &str) -> bool {
        self.inner.contains_key(id)
    }

    pub fn update_metadata(&self, id: &str, f: impl FnOnce(&mut Metadata)) {
        match self.inner.get_mut(id).as_deref_mut() {
            Some(State::Initiailized { metadata }) => {
                f(metadata);
            }
            _ => {
                warn!(%id, "no initialized state");
            }
        }
    }
}

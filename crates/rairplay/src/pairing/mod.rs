use std::sync::Arc;

use seqlock::SeqLock;

pub mod codec;
pub mod homekit;
pub mod legacy;

pub type SharedSessionKey = Arc<SeqLock<Option<SessionKey>>>;

#[derive(Debug, Clone, Copy)]
pub struct SessionKey {
    pub key_material: [u8; 32],
    pub upgrade_channel: bool,
}

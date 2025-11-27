use bytes::Bytes;

pub mod homekit;
pub mod legacy;

pub trait SessionKeyHolder: Send + Sync {
    fn set_session_key(&self, key: Bytes);
}

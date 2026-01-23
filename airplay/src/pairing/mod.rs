use bytes::Bytes;

use crate::config::Keychain;

pub mod homekit;
pub mod legacy;

pub trait SessionKeyHolder: Send + Sync {
    fn set_session_key(&self, key: Bytes);
}

pub trait KeychainHolder: Send + Sync {
    type Keychain: Keychain;

    fn keychain(&self) -> &Self::Keychain;
}

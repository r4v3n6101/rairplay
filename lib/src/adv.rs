use mac_address::{get_mac_address, MacAddress};

use crate::feats::Features;

#[derive(Debug, Clone)]
pub struct Advertisment {
    pub mac_addr: MacAddress,
    pub features: Features,
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    pub fw_version: String,
}

impl Default for Advertisment {
    fn default() -> Self {
        let mac_addr = match get_mac_address() {
            Ok(Some(addr)) => addr,
            _ => [0x9F, 0xD7, 0xAF, 0x1F, 0xD3, 0xCD].into(),
        };
        Self {
            mac_addr,
            features: Features::default(),
            manufacturer: env!("CARGO_PKG_AUTHORS").to_string(),
            model: env!("CARGO_PKG_NAME").to_string(),
            name: env!("CARGO_PKG_NAME").to_string(),
            fw_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

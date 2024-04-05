use std::fmt::{self, Display};

use mac_address::{get_mac_address, MacAddress};

use crate::feats::Features;

#[derive(Debug, Clone)]
pub struct AdvData {
    pub mac_addr: MacAddress,
    pub features: Features,
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    pub fw_version: String,
    pub pin: Option<PinCode>,
}

impl Default for AdvData {
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
            pin: None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PinCode {
    digits: [u8; 8],
}

impl PinCode {
    pub fn new(digits: [u8; 8]) -> Option<Self> {
        const INVALID_PINS: [[u8; 8]; 12] = [
            [0, 0, 0, 0, 0, 0, 0, 0],
            [1, 1, 1, 1, 1, 1, 1, 1],
            [2, 2, 2, 2, 2, 2, 2, 2],
            [3, 3, 3, 3, 3, 3, 3, 3],
            [4, 4, 4, 4, 4, 4, 4, 4],
            [5, 5, 5, 5, 5, 5, 5, 5],
            [6, 6, 6, 6, 6, 6, 6, 6],
            [7, 7, 7, 7, 7, 7, 7, 7],
            [8, 8, 8, 8, 8, 8, 8, 8],
            [9, 9, 9, 9, 9, 9, 9, 9],
            [1, 2, 3, 4, 5, 6, 7, 8],
            [8, 7, 6, 5, 4, 3, 2, 1],
        ];

        if INVALID_PINS.contains(&digits) {
            None
        } else {
            Some(Self { digits })
        }
    }

    pub fn octets(self) -> [u8; 8] {
        self.digits
    }
}

impl Display for PinCode {
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e, f, g, h] = self.digits;
        write!(fmtr, "{a}{b}-{c}{d}{e}-{f}{g}{h}")
    }
}

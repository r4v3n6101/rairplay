use serde::Serialize;

const GROUP_UUID: &str = "89581713-3fa2-4d2d-8a0e-b6840cf6b3ae";
const FEATURES: &str = "0x40018A10,0xE0300";
const MAC_ADDR: &str = "9F:D7:AF:1F:D3:CD";

#[derive(Debug, Serialize)]
pub struct Airplay2TXTRecords {
    #[serde(rename = "acl")]
    pub access_control_level: u8,
    #[serde(rename = "deviceid")]
    pub device_id: String,
    pub features: String,
    pub flags: String,
    #[serde(rename = "gcgl")]
    pub group_containing_discoverable_leader: u8,
    #[serde(rename = "gid")]
    pub group_uuid: String,
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    #[serde(rename = "protovers")]
    pub protocol_version: String,
    #[serde(rename = "rsf")]
    pub required_sender_flags: String,
    #[serde(rename = "serialNumber")]
    pub serial_number: String,
    #[serde(rename = "srcvers")]
    pub source_version: String,
    #[serde(rename = "pi")]
    pub pairing_uuid: String,
    #[serde(rename = "pk")]
    pub pubkey: String,

    // RAOP flags primarily
    #[serde(rename = "ch")]
    pub channels: u8,
    #[serde(rename = "cn")]
    pub compression: String,
}

impl Default for Airplay2TXTRecords {
    fn default() -> Self {
        Self {
            access_control_level: 0,
            device_id: MAC_ADDR.into(),
            features: FEATURES.into(),
            flags: "0x4".into(),
            group_containing_discoverable_leader: 0,
            group_uuid: GROUP_UUID.into(),
            manufacturer: env!("CARGO_PKG_AUTHORS").into(),
            model: env!("CARGO_PKG_NAME").into(),
            name: env!("CARGO_PKG_NAME").into(),
            protocol_version: "1.1".into(),
            required_sender_flags: "0x0".into(),
            serial_number: MAC_ADDR.into(),
            source_version: "366.0".into(),
            pairing_uuid: GROUP_UUID.into(),
            pubkey: Default::default(),

            channels: 2,
            compression: "0,1,2".into(),
        }
    }
}

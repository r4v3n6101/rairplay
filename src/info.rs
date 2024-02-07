#[derive(Debug, Copy, Clone)]
pub struct Pin {
    pub digits: [u8; 8],
}

#[derive(Debug, Clone)]
pub struct AppInfo {
    pub mac_addr: [u8; 6],
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    pub fw_version: String,
    pub pin: Option<Pin>,
}

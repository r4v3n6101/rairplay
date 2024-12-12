use std::{
    fmt,
    ops::{Deref, DerefMut},
};

pub struct VideoHeader([u8; Self::SIZE]);

impl fmt::Debug for VideoHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VideoHeader")
            .field("payload_len", &self.payload_len())
            .field("payload_type", &self.payload_type())
            .field("unknown6to8", &self.unknown6to8())
            .field("ntp_timestamp", &self.ntp_timestamp())
            .finish()
    }
}

impl Deref for VideoHeader {
    type Target = [u8; Self::SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VideoHeader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl VideoHeader {
    pub const SIZE: usize = 128;

    pub fn empty() -> Self {
        Self([0; Self::SIZE])
    }

    pub fn payload_len(&self) -> u32 {
        let mut len = [0; 4];
        len.copy_from_slice(&self.0[..4]);
        u32::from_le_bytes(len)
    }

    pub fn payload_type(&self) -> u16 {
        let mut payload_type = [0; 2];
        payload_type.copy_from_slice(&self.0[4..][..2]);
        u16::from_le_bytes(payload_type)
    }

    // ???
    pub fn unknown6to8(&self) -> u16 {
        let mut smth = [0; 2];
        smth.copy_from_slice(&self.0[6..][..2]);
        u16::from_le_bytes(smth)
    }

    // TODO : probably its 2xu32
    pub fn ntp_timestamp(&self) -> u64 {
        let mut ntp_ts = [0; 8];
        ntp_ts.copy_from_slice(&self.0[8..][..8]);
        u64::from_le_bytes(ntp_ts)
    }
}

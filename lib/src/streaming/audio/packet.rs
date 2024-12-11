use std::{
    fmt,
    ops::{Deref, DerefMut},
};

const AAD_LEN: usize = 8;
const TAG_LEN: usize = 16;
const NONCE_LEN: usize = 8;

pub struct RtpHeader([u8; Self::SIZE]);

impl fmt::Debug for RtpHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RtpHeader")
            .field("version", &self.version())
            .field("padding", &self.padding())
            .field("extension", &self.extension())
            .field("csrc_count", &self.csrc_count())
            .field("marker", &self.marker())
            .field("payload_type", &self.payload_type())
            .field("seqnum", &self.seqnum())
            .field("timestamp", &self.timestamp())
            .field("ssrc", &self.ssrc())
            .field("aad", &self.aad())
            .finish()
    }
}

impl Deref for RtpHeader {
    type Target = [u8; Self::SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RtpHeader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RtpHeader {
    pub const SIZE: usize = 12;

    pub fn empty() -> Self {
        Self([0; Self::SIZE])
    }

    pub fn version(&self) -> u8 {
        (self.0[0] & 0b1100_0000) >> 6
    }

    pub fn padding(&self) -> u8 {
        (self.0[0] & 0b0010_0000) >> 5
    }

    pub fn extension(&self) -> u8 {
        (self.0[0] & 0b0001_0000) >> 4
    }

    pub fn csrc_count(&self) -> u8 {
        self.0[0] & 0b0000_1111
    }

    pub fn marker(&self) -> bool {
        (self.0[1] & 0b1000_0000) >> 7 == 1
    }

    pub fn payload_type(&self) -> u8 {
        self.0[1] & 0b0111_1111
    }

    pub fn seqnum(&self) -> u16 {
        let mut seqnum = [0; 2];
        seqnum.copy_from_slice(&self.0[2..][..2]);
        u16::from_be_bytes(seqnum)
    }

    pub fn timestamp(&self) -> u32 {
        let mut timestamp = [0; 4];
        timestamp.copy_from_slice(&self.0[4..][..4]);
        u32::from_be_bytes(timestamp)
    }

    pub fn ssrc(&self) -> u32 {
        let mut ssrc = [0; 4];
        ssrc.copy_from_slice(&self.0[8..][..4]);
        u32::from_be_bytes(ssrc)
    }

    pub fn aad(&self) -> [u8; AAD_LEN] {
        let mut aad = [0; AAD_LEN];
        aad.copy_from_slice(&self.0[4..][..AAD_LEN]);
        aad
    }
}

pub struct RtpTrailer([u8; Self::SIZE]);

impl fmt::Debug for RtpTrailer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RtpTrailer")
            .field("tag", &self.tag())
            .field("nonce", &self.nonce())
            .finish()
    }
}

impl Deref for RtpTrailer {
    type Target = [u8; Self::SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RtpTrailer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RtpTrailer {
    pub const SIZE: usize = 24;

    pub fn empty() -> Self {
        Self([0; Self::SIZE])
    }

    pub fn tag(&self) -> [u8; TAG_LEN] {
        let mut tag = [0; TAG_LEN];
        tag.copy_from_slice(&self.0[..TAG_LEN]);
        tag
    }

    pub fn nonce(&self) -> [u8; NONCE_LEN] {
        let mut nonce = [0; NONCE_LEN];
        nonce.copy_from_slice(&self.0[TAG_LEN..][..NONCE_LEN]);
        nonce
    }

    pub fn padded_nonce<const N: usize>(&self) -> [u8; N] {
        assert!(
            NONCE_LEN <= N,
            "padding number must be greater or equal to nonce length"
        );

        let mut buf: [u8; N] = [0u8; N];
        buf[N - NONCE_LEN..].copy_from_slice(&self.nonce());

        buf
    }
}

pub struct RtcpHeader([u8; Self::SIZE]);

impl fmt::Debug for RtcpHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RtcpHeader")
            .field("version", &self.version())
            .field("padding", &self.padding())
            .field("reception_count", &self.reception_count())
            .field("packet_type", &self.packet_type())
            .field("packet_len", &self.packet_len())
            .finish()
    }
}

impl Deref for RtcpHeader {
    type Target = [u8; Self::SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RtcpHeader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RtcpHeader {
    pub const SIZE: usize = 4;

    pub fn empty() -> Self {
        Self([0; Self::SIZE])
    }

    pub fn version(&self) -> u8 {
        (self.0[0] & 0b1100_0000) >> 6
    }

    pub fn padding(&self) -> u8 {
        (self.0[0] & 0b0010_0000) >> 5
    }

    pub fn reception_count(&self) -> u8 {
        self.0[0] & 0b0001_1111
    }

    pub fn packet_type(&self) -> u8 {
        self.0[1]
    }

    pub fn packet_len(&self) -> u16 {
        let mut len = [0; 2];
        len.copy_from_slice(&self.0[2..][..2]);
        u16::from_be_bytes(len)
    }
}

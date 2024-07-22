use std::fmt;

use bytes::BytesMut;

const RTP_HEADER_LEN: usize = 12;
const RTP_TRAILER_LEN: usize = 24;

const AAD_LEN: usize = 8;
const TAG_LEN: usize = 16;
const NONCE_LEN: usize = 8;

pub struct RtpPacket {
    header: [u8; RTP_HEADER_LEN],
    trailer: [u8; RTP_TRAILER_LEN],
    payload: BytesMut,
}

impl fmt::Debug for RtpPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RtpPacket")
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
            .field("tag", &self.tag())
            .field("nonce", &self.nonce())
            .field("payload_len", &self.payload().len())
            .finish()
    }
}

impl RtpPacket {
    pub fn new(
        header: [u8; RTP_HEADER_LEN],
        trailer: [u8; RTP_TRAILER_LEN],
        payload: BytesMut,
    ) -> Self {
        Self {
            header,
            trailer,
            payload,
        }
    }

    pub const fn header_len() -> usize {
        RTP_HEADER_LEN
    }

    pub const fn trailer_len() -> usize {
        RTP_TRAILER_LEN
    }

    pub const fn base_len() -> usize {
        RTP_HEADER_LEN + RTP_TRAILER_LEN
    }

    pub fn version(&self) -> u8 {
        (self.header[0] & 0b1100_0000) >> 6
    }

    pub fn padding(&self) -> u8 {
        (self.header[0] & 0b0010_0000) >> 5
    }

    pub fn extension(&self) -> u8 {
        (self.header[0] & 0b0001_0000) >> 4
    }

    pub fn csrc_count(&self) -> u8 {
        self.header[0] & 0b0000_1111
    }

    pub fn marker(&self) -> bool {
        (self.header[1] & 0b1000_0000) >> 7 == 1
    }

    pub fn payload_type(&self) -> u8 {
        self.header[1] & 0b0111_1111
    }

    pub fn seqnum(&self) -> u16 {
        let mut seqnum = [0; 2];
        seqnum.copy_from_slice(&self.header[2..][..2]);
        u16::from_be_bytes(seqnum)
    }

    pub fn timestamp(&self) -> u32 {
        let mut timestamp = [0; 4];
        timestamp.copy_from_slice(&self.header[4..][..4]);
        u32::from_be_bytes(timestamp)
    }

    pub fn ssrc(&self) -> u32 {
        let mut ssrc = [0; 4];
        ssrc.copy_from_slice(&self.header[8..][..4]);
        u32::from_be_bytes(ssrc)
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.payload
    }

    pub fn aad(&self) -> [u8; AAD_LEN] {
        let mut aad = [0; AAD_LEN];
        aad.copy_from_slice(&self.header[4..][..AAD_LEN]);
        aad
    }

    pub fn tag(&self) -> [u8; TAG_LEN] {
        let mut tag = [0; TAG_LEN];
        tag.copy_from_slice(&self.trailer[..TAG_LEN]);
        tag
    }

    pub fn nonce(&self) -> [u8; NONCE_LEN] {
        let mut nonce = [0; NONCE_LEN];
        nonce.copy_from_slice(&self.trailer[TAG_LEN..][..NONCE_LEN]);
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

use bytes::{Bytes, BytesMut};

const RTP_HEADER_LEN: usize = 12;
const RTP_TRAILER_LEN: usize = 24;

const AAD_LEN: usize = 8;
const TAG_LEN: usize = 16;
const NONCE_LEN: usize = 8;

#[derive(Debug, Clone)]
pub struct RtpPacket {
    header: Bytes,
    payload: BytesMut,
}

#[derive(Debug, Clone)]
pub struct BufferedRtpPacket {
    inner: RtpPacket,
    trailer: Bytes,
}

impl RtpPacket {
    pub fn new(mut payload: BytesMut) -> Self {
        let header = payload.split_to(RTP_HEADER_LEN).freeze();
        Self { header, payload }
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.payload
    }
}

impl BufferedRtpPacket {
    pub fn new(mut payload: BytesMut) -> Self {
        let trailer = payload.split_off(payload.len() - RTP_TRAILER_LEN).freeze();
        Self {
            inner: RtpPacket::new(payload),
            trailer,
        }
    }

    pub fn rtp(&self) -> &RtpPacket {
        &self.inner
    }

    pub fn rtp_mut(&mut self) -> &mut RtpPacket {
        &mut self.inner
    }

    pub fn aad(&self) -> [u8; AAD_LEN] {
        self.inner.header[4..][..AAD_LEN].try_into().unwrap()
    }

    pub fn tag(&self) -> [u8; TAG_LEN] {
        self.trailer[..TAG_LEN].try_into().unwrap()
    }

    pub fn nonce(&self) -> [u8; NONCE_LEN] {
        self.trailer[TAG_LEN..][..NONCE_LEN].try_into().unwrap()
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

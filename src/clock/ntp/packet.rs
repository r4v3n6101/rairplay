impl NtpTimestamp {
    pub fn new(buf: [u8; 8]) -> Self {
        Self {
            seconds: u32::from_be_bytes(buf[0..4].try_into().unwrap()),
            fraction: u32::from_be_bytes(buf[4..8].try_into().unwrap()),
        }
    }

    pub fn as_bytes(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[0..4].copy_from_slice(&self.seconds.to_be_bytes());
        buf[4..8].copy_from_slice(&self.fraction.to_be_bytes());
        buf
    }
}

#[derive(Debug, Copy, Clone)]
pub struct NtpPacket {
    pub leap_indicator: u8,
    pub version: u8,
    pub mode: u8,
    pub stratum: u8,
    pub poll: u8,
    pub precision: u8,
    pub root_delay: u32,
    pub root_dispersion: u32,
    pub ref_id: u32,
    pub ref_timestamp: NtpTimestamp,
    pub originate_timestamp: NtpTimestamp,
    pub rx_timestamp: NtpTimestamp,
    pub tx_timestamp: NtpTimestamp,
}

impl NtpPacket {
    pub fn new(buf: [u8; 48]) -> Self {
        Self {
            leap_indicator: (buf[0] & 0b1100_0000) >> 6,
            version: (buf[0] & 0b0011_1000) >> 3,
            mode: buf[0] & 0b0000_0111,
            stratum: buf[1],
            poll: buf[2],
            precision: buf[3],
            root_delay: u32::from_be_bytes(buf[4..8].try_into().unwrap()),
            root_dispersion: u32::from_be_bytes(buf[8..12].try_into().unwrap()),
            ref_id: u32::from_be_bytes(buf[12..16].try_into().unwrap()),
            ref_timestamp: NtpTimestamp::new(buf[16..24].try_into().unwrap()),
            originate_timestamp: NtpTimestamp::new(buf[24..32].try_into().unwrap()),
            rx_timestamp: NtpTimestamp::new(buf[32..40].try_into().unwrap()),
            tx_timestamp: NtpTimestamp::new(buf[40..48].try_into().unwrap()),
        }
    }

    pub fn as_bytes(&self) -> [u8; 48] {
        let mut buf = [0u8; 48];
        buf[0] |= (self.leap_indicator & 0b11) << 6;
        buf[0] |= (self.version & 0b111) << 3;
        buf[0] |= self.mode & 0b111;
        buf[1] = self.stratum;
        buf[2] = self.poll;
        buf[3] = self.precision;
        buf[4..8].copy_from_slice(&self.root_delay.to_be_bytes());
        buf[8..12].copy_from_slice(&self.root_dispersion.to_be_bytes());
        buf[12..16].copy_from_slice(&self.ref_id.to_be_bytes());
        buf[16..24].copy_from_slice(&self.ref_timestamp.as_bytes());
        buf[24..32].copy_from_slice(&self.originate_timestamp.as_bytes());
        buf[32..40].copy_from_slice(&self.rx_timestamp.as_bytes());
        buf[40..48].copy_from_slice(&self.tx_timestamp.as_bytes());
        buf
    }
}

pub struct VideoPacket {
    pub _unknown_field: u16,
    pub ntp_timestamp: NtpTimestamp,
    pub payload: VideoPacketPayload,
}

pub enum VideoPacketPayload {
    // TODO : something for raw bytes, BytesMut or Vec<u8>, idk
    Bitstream(()),
    // TODO : same type as above, it's avcC format
    CodecData(()),
    Heartbeat,
}

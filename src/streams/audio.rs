use std::net::UdpSocket;

use openssl::aes::AesKey;
use symphonia_core::codecs::CodecParameters;

const FRAMES_IN_BUF: usize = 32;

pub struct AudioStream {
    codec_params: CodecParameters,
    aes_key_iv: Option<(AesKey, Vec<u8>)>,
    socket: UdpSocket,
}

impl AudioStream {
    pub fn new(codec_params: CodecParameters, aes_key_iv: Option<(AesKey, Vec<u8>)>) -> Self {
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap(); // TODO
        Self {
            codec_params,
            aes_key_iv,
            socket,
        }
    }

    pub async fn serve(self) {
    }
}

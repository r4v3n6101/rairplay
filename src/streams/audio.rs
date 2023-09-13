use asynchronous_codec::FramedRead;
use futures::{io::empty, StreamExt};
use openssl::aes::AesKey;
use symphonia_core::codecs::CodecParameters;
use tracing::warn;

use crate::codecs::rtp::RtpCodec;

const FRAMES_IN_BUF: usize = 32;

pub struct AudioStream {
    codec_params: CodecParameters,
    aes_key_iv: Option<(AesKey, Vec<u8>)>,
}

impl AudioStream {
    pub fn new(codec_params: CodecParameters, aes_key_iv: Option<(AesKey, Vec<u8>)>) -> Self {
        Self {
            codec_params,
            aes_key_iv,
        }
    }

    pub async fn serve(self) {
        let mut packet_stream = FramedRead::new(empty(), RtpCodec)
            .filter_map(|res| async {
                match res {
                    Ok(packet) => return Some(packet),
                    Err(e) => {
                        warn!(%e, "invalid rtp packet");
                        None
                    }
                }
            })
            .ready_chunks(FRAMES_IN_BUF);
    }
}

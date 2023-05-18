use std::sync::Weak;

use futures::{Sink, StreamExt};
use tokio::net::UdpSocket;
use tokio_util::udp::UdpFramed;
use tracing::{instrument, trace, warn};

use crate::session::ClientSession;

use super::codec::RtpCodec;

// TODO : think about u8 instead of sinking Vec
pub trait AudioSink: Sink<Vec<u8>> {
    fn create(sample_rate: u32, sample_size: u16, channels: u8) -> Self;
    // TODO : remove mutability?
    fn set_volume(&mut self, value: f32);
    fn get_volume(&self) -> f32;
}

// TODO : not key, but decryptor (as fairplay isn't aes)
#[instrument]
pub async fn forward_decrypted_audio(socket: UdpSocket, session: Weak<ClientSession>) {
    let mut rx = UdpFramed::new(socket, RtpCodec);
    while let Some(res) = rx.next().await {
        match res {
            Ok((packet, _)) => {
                trace!("received rtp audio packet");
                let buf = vec![0; packet.payload.len()];

                trace!(len = buf.len(), "packet decryption");
                // TODO : key.decrypt(&packet.payload, &mut buf);
            }
            Err(err) => {
                // TODO : retry it via control?
                warn!(%err, "corrupted packet, skipping it");
                continue;
            }
        }
    }
}

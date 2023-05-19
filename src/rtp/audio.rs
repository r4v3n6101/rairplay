use std::{marker::Unpin, sync::Weak};

use futures::{SinkExt, StreamExt};
use tokio::{net::UdpSocket, sync::RwLock};
use tokio_util::udp::UdpFramed;
use tracing::{info, instrument, trace, warn};

use crate::audio::{AudioCipher, AudioSink};

use super::codec::RtpCodec;

#[instrument]
pub async fn forward_decrypted_audio<A, C>(
    socket: UdpSocket,
    audio_sink: Weak<RwLock<A>>,
    mut cipher: Option<C>,
) where
    A: AudioSink + Unpin,
    C: AudioCipher,
{
    let mut rx = UdpFramed::new(socket, RtpCodec);
    loop {
        let Some(audio_sink) = audio_sink.upgrade() else {
            info!("audio sink disconnected from forwarder");
            break;
        };

        let Some(res) = rx.next().await else {
            info!("audio socket closed");
            break;
        };

        match res {
            Ok((packet, _)) => {
                trace!(len = packet.payload.len(), "received rtp audio packet");
                // TODO : try not allocate and instead just feed stream
                let buf = if let Some(ref mut cipher) = cipher {
                    let mut buf = vec![0; packet.payload.len()];
                    cipher.decrypt(&packet.payload, &mut buf);
                    // TODO : print afterall size, that shouldn't be the same
                    trace!("packet decrypted");
                    buf
                } else {
                    packet.payload.to_vec()
                };

                audio_sink.write().await.feed(buf).await;
            }
            Err(err) => {
                // TODO : retry via control socket?
                warn!(%err, "corrupted packet, skipping it");
                continue;
            }
        }
    }
}

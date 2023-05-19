use std::{
    io,
    net::{IpAddr, SocketAddr, UdpSocket},
    sync::Weak,
};

use tokio::sync::RwLock;
use tracing::{debug, instrument};

use crate::audio::{AudioCipher, AudioSink};

use self::audio::forward_decrypted_audio;

pub mod audio;
pub mod codec;

#[derive(Debug)]
pub struct RtpTransport {
    pub audio_port: u16,
    pub control_port: u16,
    pub timing_port: u16,
}

#[instrument]
pub fn spawn_listener<A, C>(
    bind_addr: IpAddr,
    remote_addr: IpAddr,
    audio_sink: Weak<RwLock<A>>,
    cipher: Option<C>,
) -> io::Result<RtpTransport>
where
    A: AudioSink + Send + Sync + Unpin + 'static,
    C: AudioCipher + Send + 'static,
{
    let create_chan = move |port| {
        let sock = UdpSocket::bind(SocketAddr::new(bind_addr, 0))?;
        debug!(?sock, "created udp socket");
        if let Some(port) = port {
            sock.connect(SocketAddr::new(remote_addr, port))?;
            debug!(port, "connected to remote port");
        }

        Ok::<_, io::Error>(sock)
    };

    let audio = create_chan(None)?;
    // TODO : replace with actual ports, currently unused
    let control = create_chan(None)?;
    let timing = create_chan(None)?;

    let transport = RtpTransport {
        audio_port: audio.local_addr()?.port(),
        control_port: control.local_addr()?.port(),
        timing_port: timing.local_addr()?.port(),
    };

    tokio::spawn(forward_decrypted_audio(
        tokio::net::UdpSocket::from_std(audio)?,
        Weak::clone(&audio_sink),
        cipher,
    ));

    Ok(transport)
}

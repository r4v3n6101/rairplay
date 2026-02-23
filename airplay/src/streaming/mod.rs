use std::{
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use derivative::Derivative;
use tokio::net::{TcpListener, UdpSocket};

use crate::{
    crypto::{AesIv128, AesKey128, ChaCha20Poly1305Key},
    pairing::SessionKey,
    playback::{ChannelHandle, audio::AudioStream, video::VideoStream},
    util::sync::WakerFlag,
};

mod processing;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct EventChannel {
    local_addr: SocketAddr,
    #[derivative(Debug = "ignore")]
    waker_flag: Arc<WakerFlag>,
}

#[derive(Debug)]
pub struct AudioRealtimeChannel {
    pub local_data_addr: SocketAddr,
    pub local_control_addr: SocketAddr,
}

#[derive(Debug)]
pub struct AudioBufferedChannel {
    pub local_addr: SocketAddr,
    pub audio_buf_size: u32,
}

#[derive(Debug)]
pub struct VideoChannel {
    pub local_addr: SocketAddr,
}

#[derive(Default)]
pub struct SharedData {
    pub waker_flag: WakerFlag,
}

#[derive(Debug)]
pub struct EncryptionMaterial {
    pub stream_connection_id: Option<u64>,
    pub chacha_key: Option<ChaCha20Poly1305Key>,
    pub session_key: Option<SessionKey>,
    pub aeskey: Option<AesKey128>,
    pub aesiv: Option<AesIv128>,
}

impl EventChannel {
    #[tracing::instrument(ret, err)]
    pub async fn create(bind_addr: IpAddr) -> io::Result<Self> {
        let listener = TcpListener::bind(SocketAddr::new(bind_addr, 0)).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!(%local_addr, "created new listener");

        let waker_flag = Arc::new(WakerFlag::default());
        let wf = Arc::clone(&waker_flag);
        tokio::spawn(async move {
            tokio::select! {
                () = &*wf => {}
                () = processing::event_processor(listener) => {}
            };
            tracing::info!("event listener done");
        });

        Ok(EventChannel {
            local_addr,
            waker_flag,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl AudioBufferedChannel {
    #[tracing::instrument(ret, err, skip(shared_data, stream))]
    pub async fn create(
        bind_addr: IpAddr,
        expected_remote_addr: IpAddr,
        shared_data: Arc<SharedData>,
        stream: impl AudioStream,
        audio_buf_size: u32,
        keys: EncryptionMaterial,
    ) -> io::Result<Self> {
        let encryption = processing::Encryption::try_from(keys)?;

        let listener = TcpListener::bind(SocketAddr::new(bind_addr, 0)).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!(%local_addr, "created new listener");

        tokio::spawn(async move {
            let task = async {
                loop {
                    return match listener.accept().await {
                        Ok((tcp_stream, stream_remote_addr)) => {
                            if expected_remote_addr != stream_remote_addr.ip() {
                                tracing::debug!(%stream_remote_addr, "skip invalid connection");
                                continue;
                            }

                            processing::audio_buffered_processor(
                                tcp_stream,
                                &stream,
                                audio_buf_size,
                                encryption,
                            )
                            .await
                        }
                        Err(err) => Err(err),
                    };
                }
            };

            tokio::select! {
                () = &shared_data.waker_flag => {},
                res = task => match remap_io_error_if_needed(res) {
                    Ok(()) => stream.on_ok(),
                    Err(err) => stream.on_err(err.into()),
                }
            }
        });

        Ok(Self {
            local_addr,
            audio_buf_size,
        })
    }
}

impl AudioRealtimeChannel {
    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(ret, err, skip(shared_data, stream))]
    pub async fn create(
        bind_addr: IpAddr,
        expected_remote_addr: IpAddr,
        shared_data: Arc<SharedData>,
        stream: impl AudioStream,
        audio_buf_size: u32,
        keys: EncryptionMaterial,
    ) -> io::Result<Self> {
        let encryption = processing::Encryption::try_from(keys)?;

        let data_socket = UdpSocket::bind(SocketAddr::new(bind_addr, 0)).await?;
        let control_socket = UdpSocket::bind(SocketAddr::new(bind_addr, 0)).await?;

        let local_data_addr = data_socket.local_addr()?;
        tracing::info!(%local_data_addr, "created new socket");

        let local_control_addr = control_socket.local_addr()?;
        tracing::info!(%local_control_addr, "created new socket");

        tokio::spawn(async move {
            let task = async {
                let data = processing::audio_realtime_processor(
                    expected_remote_addr,
                    data_socket,
                    &stream,
                    audio_buf_size,
                    encryption,
                );
                let control = processing::control_processor(expected_remote_addr, control_socket);

                let (first, second) = tokio::join!(data, control);
                first.or(second)
            };

            tokio::select! {
                () = &shared_data.waker_flag => {},
                res = task => match remap_io_error_if_needed(res) {
                    Ok(()) => stream.on_ok(),
                    Err(err) => stream.on_err(err.into()),
                }
            }
        });

        Ok(Self {
            local_data_addr,
            local_control_addr,
        })
    }
}

impl VideoChannel {
    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(ret, err, skip(shared_data, stream))]
    pub async fn create(
        bind_addr: IpAddr,
        expected_remote_addr: IpAddr,
        shared_data: Arc<SharedData>,
        stream: impl VideoStream,
        video_buf_size: u32,
        keys: EncryptionMaterial,
    ) -> io::Result<Self> {
        let encryption = processing::Encryption::try_from(keys)?;

        let listener = TcpListener::bind(SocketAddr::new(bind_addr, 0)).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!(%local_addr, "created new listener");

        tokio::spawn(async move {
            let task = async {
                loop {
                    return match listener.accept().await {
                        Ok((tcp_stream, stream_remote_addr)) => {
                            if expected_remote_addr != stream_remote_addr.ip() {
                                tracing::debug!(%stream_remote_addr, "skip invalid connection");
                                continue;
                            }

                            processing::video_processor(
                                tcp_stream,
                                &stream,
                                video_buf_size,
                                encryption,
                            )
                            .await
                        }
                        Err(err) => Err(err),
                    };
                }
            };

            tokio::select! {
                () = &shared_data.waker_flag => {},
                res = task => match remap_io_error_if_needed(res) {
                    Ok(()) => stream.on_ok(),
                    Err(err) => stream.on_err(err.into()),
                }
            }
        });

        Ok(Self { local_addr })
    }
}

impl Drop for EventChannel {
    fn drop(&mut self) {
        self.waker_flag.set_and_wake();
    }
}

impl ChannelHandle for SharedData {
    fn close(&self) {
        self.waker_flag.set_and_wake();
    }
}

fn remap_io_error_if_needed(res: io::Result<()>) -> io::Result<()> {
    match res {
        Ok(()) => Ok(()),
        Err(err)
            if matches!(
                err.kind(),
                io::ErrorKind::UnexpectedEof
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::ConnectionReset
            ) =>
        {
            Ok(())
        }
        Err(err) => Err(err),
    }
}

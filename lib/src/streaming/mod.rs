use std::{io, net::SocketAddr, sync::Arc};

use tokio::{
    net::{TcpListener, ToSocketAddrs, UdpSocket},
    sync::Notify,
};

use crate::{
    crypto::streaming::{AudioBufferedCipher, AudioRealtimeCipher, VideoCipher},
    playback::{audio::AudioStream, video::VideoStream, ChannelHandle, Stream},
    util::{io::remap_io_error_if_needed, sync::CancellationHandle},
};

mod processing;

pub struct EventChannel {
    local_addr: SocketAddr,
    shutdown: Arc<Notify>,
}

pub struct AudioRealtimeChannel {
    pub local_data_addr: SocketAddr,
    pub local_control_addr: SocketAddr,
}

pub struct AudioBufferedChannel {
    pub local_addr: SocketAddr,
    pub audio_buf_size: u32,
}

pub struct VideoChannel {
    pub local_addr: SocketAddr,
}

#[derive(Default)]
pub struct SharedData {
    pub handle: CancellationHandle,
}

impl EventChannel {
    pub async fn create(bind_addr: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;
        let notify = Arc::new(Notify::new());

        let notify1 = Arc::clone(&notify);
        tokio::spawn(async move {
            tokio::select! {
                () = notify1.notified() => {}
                () = processing::event_processor(listener, local_addr) => {}
            };
            tracing::info!("event listener done");
        });

        Ok(EventChannel {
            local_addr,
            shutdown: notify,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl AudioRealtimeChannel {
    pub async fn create(
        data_bind_addr: impl ToSocketAddrs,
        control_bind_addr: impl ToSocketAddrs,
        audio_buf_size: u32,
        shared_data: Arc<SharedData>,
        cipher: AudioRealtimeCipher,
        stream: impl AudioStream,
    ) -> io::Result<Self> {
        let data_socket = UdpSocket::bind(data_bind_addr).await?;
        let control_socket = UdpSocket::bind(control_bind_addr).await?;

        let local_data_addr = data_socket.local_addr()?;
        let local_control_addr = control_socket.local_addr()?;

        tokio::spawn(async move {
            let task = async {
                let data = processing::audio_realtime_processor(
                    data_socket,
                    audio_buf_size,
                    cipher,
                    &stream,
                );
                let control = processing::control_processor(control_socket);

                let (first, second) = tokio::join!(data, control);
                first.or(second)
            };

            // tokio will handle it with boxing
            #[allow(clippy::large_futures)]
            match remap_io_error_if_needed(shared_data.handle.wrap_task(task).await) {
                Ok(()) => stream.on_ok(),
                Err(err) => stream.on_err(err.into()),
            }
        });

        Ok(Self {
            local_data_addr,
            local_control_addr,
        })
    }
}

impl AudioBufferedChannel {
    pub async fn create(
        bind_addr: impl ToSocketAddrs,
        audio_buf_size: u32,
        shared_data: Arc<SharedData>,
        cipher: AudioBufferedCipher,
        stream: impl AudioStream,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        tokio::spawn(async move {
            let task = async {
                match listener.accept().await {
                    Ok((tcp_stream, _)) => {
                        processing::audio_buffered_processor(
                            audio_buf_size,
                            tcp_stream,
                            cipher,
                            &stream,
                        )
                        .await
                    }
                    Err(err) => Err(err),
                }
            };

            match remap_io_error_if_needed(shared_data.handle.wrap_task(task).await) {
                Ok(()) => stream.on_ok(),
                Err(err) => stream.on_err(err.into()),
            }
        });

        Ok(Self {
            local_addr,
            audio_buf_size,
        })
    }
}

impl VideoChannel {
    pub async fn create(
        bind_addr: impl ToSocketAddrs,
        video_buf_size: u32,
        shared_data: Arc<SharedData>,
        cipher: VideoCipher,
        stream: impl VideoStream,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        tokio::spawn(async move {
            let task = async {
                match listener.accept().await {
                    Ok((tcp_stream, _)) => {
                        processing::video_processor(video_buf_size, tcp_stream, cipher, &stream)
                            .await
                    }
                    Err(err) => Err(err),
                }
            };
            match remap_io_error_if_needed(shared_data.handle.wrap_task(task).await) {
                Ok(()) => stream.on_ok(),
                Err(err) => stream.on_err(err.into()),
            }
        });

        Ok(Self { local_addr })
    }
}

impl Drop for EventChannel {
    fn drop(&mut self) {
        self.shutdown.notify_waiters();
    }
}

impl ChannelHandle for SharedData {
    fn close(&self) {
        self.handle.close();
    }
}

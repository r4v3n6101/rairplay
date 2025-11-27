use std::{io, net::SocketAddr, sync::Arc};

use derivative::Derivative;
use tokio::net::{TcpListener, ToSocketAddrs, UdpSocket};

use crate::{
    crypto::{AesIv128, AesKey128, ChaCha20Poly1305Key},
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

impl EventChannel {
    #[tracing::instrument(ret, err, skip(bind_addr))]
    pub async fn create(bind_addr: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;
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

impl AudioRealtimeChannel {
    #[tracing::instrument(ret, err, skip(data_bind_addr, control_bind_addr, shared_data, stream))]
    pub async fn create(
        data_bind_addr: impl ToSocketAddrs,
        control_bind_addr: impl ToSocketAddrs,
        shared_data: Arc<SharedData>,
        stream: impl AudioStream,
        audio_buf_size: u32,
        key: AesKey128,
        iv: AesIv128,
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
                    key,
                    iv,
                    &stream,
                );
                let control = processing::control_processor(control_socket);

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

impl AudioBufferedChannel {
    #[tracing::instrument(ret, err, skip(bind_addr, shared_data, stream))]
    pub async fn create(
        bind_addr: impl ToSocketAddrs,
        shared_data: Arc<SharedData>,
        stream: impl AudioStream,
        audio_buf_size: u32,
        key: ChaCha20Poly1305Key,
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
                            key,
                            &stream,
                        )
                        .await
                    }
                    Err(err) => Err(err),
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

impl VideoChannel {
    #[tracing::instrument(ret, err, skip(bind_addr, shared_data, stream))]
    pub async fn create(
        bind_addr: impl ToSocketAddrs,
        shared_data: Arc<SharedData>,
        stream: impl VideoStream,
        video_buf_size: u32,
        key: AesKey128,
        stream_connection_id: u64,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        tokio::spawn(async move {
            let task = async {
                match listener.accept().await {
                    Ok((tcp_stream, _)) => {
                        processing::video_processor(
                            video_buf_size,
                            tcp_stream,
                            key,
                            stream_connection_id,
                            &stream,
                        )
                        .await
                    }
                    Err(err) => Err(err),
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

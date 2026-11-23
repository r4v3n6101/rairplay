use std::{
    error::Error,
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use derivative::Derivative;
use tokio::net::{TcpListener, TcpStream, UdpSocket};

use crate::{
    crypto::{AesIv128, AesKey128, ChaCha20Poly1305Key},
    pairing::SessionKey,
    playback::{
        ChannelHandle, EndReason, Stream,
        audio::{AudioDevice, AudioParams},
        video::{VideoDevice, VideoParams},
    },
};

mod processing;
mod sync;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct EventChannel {
    local_addr: SocketAddr,
    #[derivative(Debug = "ignore")]
    waker_flag: Arc<sync::WakerFlag>,
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
    pub waker_flag: sync::WakerFlag,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ChannelSetup<'a, D> {
    pub id: u64,
    pub bind_addr: IpAddr,
    pub expected_remote_addr: IpAddr,
    #[derivative(Debug = "ignore")]
    pub shared_data: Arc<SharedData>,
    #[derivative(Debug = "ignore")]
    pub device: &'a D,
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

        let waker_flag = Arc::new(sync::WakerFlag::default());
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
    #[tracing::instrument(ret, err, skip(keys))]
    pub async fn create(
        channel: ChannelSetup<'_, impl AudioDevice>,
        params: AudioParams,
        audio_buf_size: u32,
        keys: EncryptionMaterial,
    ) -> Result<Self, Box<dyn Error>> {
        let encryption = processing::Encryption::try_from(keys)?;

        let listener = TcpListener::bind(SocketAddr::new(channel.bind_addr, 0)).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!(%local_addr, "created new listener");

        let handle = Arc::downgrade(&channel.shared_data);
        let stream = channel.device.create(channel.id, params, handle).await?;
        tracing::trace!("new stream opened");

        tokio::spawn(async move {
            let task = async {
                let tcp_stream = accept_from(&listener, channel.expected_remote_addr).await?;
                processing::audio_buffered_processor(
                    tcp_stream,
                    &stream,
                    audio_buf_size,
                    encryption,
                )
                .await
            };

            let reason = tokio::select! {
                biased;
                () = &channel.shared_data.waker_flag => EndReason::Cancelled,
                res = task => transport_end_reason(res),
            };
            stream.on_end(reason);
        });

        Ok(Self {
            local_addr,
            audio_buf_size,
        })
    }
}

impl AudioRealtimeChannel {
    #[tracing::instrument(ret, err, skip(keys))]
    pub async fn create(
        channel: ChannelSetup<'_, impl AudioDevice>,
        params: AudioParams,
        audio_buf_size: u32,
        keys: EncryptionMaterial,
    ) -> Result<Self, Box<dyn Error>> {
        let encryption = processing::Encryption::try_from(keys)?;

        let bind_addr = SocketAddr::new(channel.bind_addr, 0);
        let data_socket = UdpSocket::bind(bind_addr).await?;
        let control_socket = UdpSocket::bind(bind_addr).await?;

        let local_data_addr = data_socket.local_addr()?;
        tracing::info!(%local_data_addr, "created new socket");

        let local_control_addr = control_socket.local_addr()?;
        tracing::info!(%local_control_addr, "created new socket");

        let handle = Arc::downgrade(&channel.shared_data);
        let stream = channel.device.create(channel.id, params, handle).await?;
        tracing::trace!("new stream opened");

        tokio::spawn(async move {
            let task = async {
                let data = processing::audio_realtime_processor(
                    channel.expected_remote_addr,
                    data_socket,
                    &stream,
                    audio_buf_size,
                    encryption,
                );
                let control =
                    processing::control_processor(channel.expected_remote_addr, control_socket);

                tokio::try_join!(data, control)?;

                Ok(())
            };

            let reason = tokio::select! {
                biased;
                () = &channel.shared_data.waker_flag => EndReason::Cancelled,
                res = task => transport_end_reason(res),
            };
            stream.on_end(reason);
        });

        Ok(Self {
            local_data_addr,
            local_control_addr,
        })
    }
}

impl VideoChannel {
    #[tracing::instrument(ret, err, skip(keys))]
    pub async fn create(
        channel: ChannelSetup<'_, impl VideoDevice>,
        params: VideoParams,
        video_buf_size: u32,
        keys: EncryptionMaterial,
    ) -> Result<Self, Box<dyn Error>> {
        let encryption = processing::Encryption::try_from(keys)?;

        let listener = TcpListener::bind(SocketAddr::new(channel.bind_addr, 0)).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!(%local_addr, "created new listener");

        let handle = Arc::downgrade(&channel.shared_data);
        let stream = channel.device.create(channel.id, params, handle).await?;
        tracing::trace!("new stream opened");

        tokio::spawn(async move {
            let task = async {
                let tcp_stream = accept_from(&listener, channel.expected_remote_addr).await?;
                processing::video_processor(tcp_stream, &stream, video_buf_size, encryption).await
            };

            let reason = tokio::select! {
                biased;
                () = &channel.shared_data.waker_flag => EndReason::Cancelled,
                res = task => transport_end_reason(res),
            };
            stream.on_end(reason);
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

async fn accept_from(
    listener: &TcpListener,
    expected_remote_addr: IpAddr,
) -> io::Result<TcpStream> {
    loop {
        match listener.accept().await? {
            (stream, stream_remote_addr) if stream_remote_addr.ip() == expected_remote_addr => {
                return Ok(stream);
            }
            (_, stream_remote_addr) => {
                tracing::debug!(%stream_remote_addr, "skip invalid connection");
            }
        }
    }
}

fn transport_end_reason(res: io::Result<()>) -> EndReason {
    match res {
        Ok(()) => EndReason::Finished,
        Err(err)
            if matches!(
                err.kind(),
                io::ErrorKind::UnexpectedEof
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::ConnectionReset
            ) =>
        {
            EndReason::Finished
        }
        Err(err) => EndReason::Transport(err),
    }
}

use std::sync::{Arc, Weak, atomic::Ordering};

use axum::{
    extract::{ConnectInfo, State},
    response::IntoResponse,
};
use bytes::Bytes;
use http::{header::CONTENT_TYPE, status::StatusCode};

use super::{
    dto::{
        AudioBufferedRequest, AudioRealtimeRequest, Display, InfoResponse, SenderInfo,
        SetupRequest, SetupResponse, StreamRequest, StreamResponse, StreamType, Teardown,
        VideoRequest,
    },
    extractor::BinaryPlist,
    state::ServiceState,
    transport::Connection,
};
use crate::{
    crypto::{AesIv128, ChaCha20Poly1305Key, sha512_two_step},
    playback::{
        ChannelHandle,
        audio::{AUDIO_FORMATS, AudioDevice, AudioParams},
        video::{VideoDevice, VideoParams},
    },
    streaming::{
        AudioBufferedChannel, AudioRealtimeChannel, EventChannel, SharedData, VideoChannel,
    },
};

mod fairplay;

#[tracing::instrument(level = "TRACE")]
pub async fn generic(bytes: Bytes) {}

#[tracing::instrument(level = "DEBUG", ret, skip(state))]
pub async fn info<A, V, K>(
    State(state): State<Arc<ServiceState<A, V, K>>>,
) -> BinaryPlist<InfoResponse> {
    const PROTOVERS: &str = "1.1";
    const SRCVERS: &str = "770.8.1";

    let response = InfoResponse {
        device_id: state.config.mac_addr,
        mac_addr: state.config.mac_addr,
        features: state.config.features.bits(),
        protocol_version: PROTOVERS.to_string(),
        source_version: SRCVERS.to_string(),

        manufacturer: state.config.manufacturer.clone(),
        model: state.config.model.clone(),
        name: state.config.name.clone(),

        // Seems like clients don't respect other displays and pick by maximum resolution
        displays: vec![Display {
            width: state.config.video.width,
            height: state.config.video.height,
            uuid: format!("{}_display", state.config.name),
            max_fps: state.config.video.fps,
            features: 2,
        }],
    };

    BinaryPlist(response)
}

#[tracing::instrument(level = "DEBUG", ret(level = "TRACE"), err, skip(state))]
pub async fn fp_setup<A, V, K>(
    State(state): State<Arc<ServiceState<A, V, K>>>,
    body: Bytes,
) -> Result<Vec<u8>, StatusCode> {
    fairplay::decode_buf(&body)
        .inspect(|_| {
            let Ok(msg) = <[u8; _]>::try_from(&body[..]) else {
                return;
            };

            state.fp_last_msg.lock_write().replace(msg);
            tracing::trace!("fairplay3 last message is saved");
        })
        .inspect_err(|err| tracing::error!(%err, "failed to decode fairplay"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tracing::instrument(level = "DEBUG", ret, err, skip(state))]
pub async fn get_parameter<A: AudioDevice, V, K>(
    State(state): State<Arc<ServiceState<A, V, K>>>,
    body: String,
) -> Result<impl IntoResponse, StatusCode> {
    match body.as_str() {
        "volume\r\n" => {
            let volume = state.config.audio.device.get_volume();
            Ok((
                [(CONTENT_TYPE, "text/parameters")],
                format!("volume: {volume}\r\n"),
            ))
        }
        param => {
            tracing::error!(?param, "unimplemented parameter");
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    }
}

pub async fn set_parameter(_body: Bytes) {}

#[tracing::instrument(level = "DEBUG", skip(state))]
pub async fn teardown<A, V, K>(
    State(state): State<Arc<ServiceState<A, V, K>>>,
    BinaryPlist(req): BinaryPlist<Teardown>,
) {
    let mut stream_channels = state.stream_channels.lock().unwrap();
    if let Some(requests) = req.requests {
        for req in requests {
            if let Some(id) = req.id {
                stream_channels
                    .iter()
                    .filter(|((i, _), _)| *i == id)
                    .for_each(|(_, chan)| chan.close());
                stream_channels.retain(|(i, _), _| *i != id);
                tracing::info!(%id, "teardown stream");
            } else {
                stream_channels
                    .iter()
                    .filter(|((_, ty), _)| *ty == req.ty as u32)
                    .for_each(|(_, chan)| chan.close());
                stream_channels.retain(|(_, ty), _| *ty != req.ty as u32);
                tracing::info!(type=?req.ty, "teardown stream");
            }
        }
    } else {
        let num = stream_channels.len();
        stream_channels.iter().for_each(|(_, chan)| chan.close());
        stream_channels.clear();
        tracing::info!(%num, "teardown all streams");
    }
}

pub async fn setup<A: AudioDevice, V: VideoDevice, K>(
    State(state): State<Arc<ServiceState<A, V, K>>>,
    ConnectInfo(conn): ConnectInfo<Connection>,
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> Result<BinaryPlist<SetupResponse>, StatusCode> {
    match req {
        SetupRequest::SenderInfo(info) => setup_info(&state, &conn, *info).await,
        SetupRequest::Streams { requests } => setup_streams(&state, &conn, requests).await,
    }
}

#[tracing::instrument(level = "DEBUG", ret, err, skip(state))]
async fn setup_info<A, V, K>(
    state: &ServiceState<A, V, K>,
    conn: &Connection,
    SenderInfo { ekey, eiv, .. }: SenderInfo,
) -> Result<BinaryPlist<SetupResponse>, StatusCode> {
    let mut lock = state.event_channel.lock().await;
    let event_channel = match &mut *lock {
        Some(chan) => chan,
        event_channel @ None => EventChannel::create(conn.bind_addr().unwrap())
            .await
            .map(|chan| event_channel.insert(chan))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    };

    if let Some(ekey) = ekey
        && let Some(eiv) = eiv
    {
        tracing::trace!(?ekey, ?eiv, "ekev & eiv detected");

        let Some(session_key) = conn.session_key.read() else {
            tracing::error!("must be paired and have session_key");
            return Err(StatusCode::BAD_REQUEST);
        };
        let Ok(eiv) = AesIv128::try_from(eiv.as_ref()) else {
            tracing::error!(len=%eiv.len(), "invalid length of passed iv");
            return Err(StatusCode::BAD_REQUEST);
        };
        let Some(fp_last_msg) = state.fp_last_msg.read() else {
            tracing::error!("fairplay3 handshake must be present");
            return Err(StatusCode::BAD_REQUEST);
        };

        let aes_key = fairplay::decrypt_key(fp_last_msg, &ekey);
        tracing::trace!(?aes_key, ?fp_last_msg, "aes key decrypted with fairplay");

        let aes_key = sha512_two_step(&aes_key, &session_key.key_material);
        tracing::trace!(
            ?aes_key,
            ?session_key,
            "additional hashing with pairing's shared secret"
        );

        state.ekey.lock_write().replace(aes_key);
        state.eiv.lock_write().replace(eiv);
    }

    // TODO : log more info from SenderInfo

    Ok(BinaryPlist(SetupResponse::Info {
        event_port: event_channel.local_addr().port(),
        timing_port: 0,
    }))
}

#[tracing::instrument(level = "DEBUG", skip_all)]
async fn setup_streams<A: AudioDevice, V: VideoDevice, K>(
    state: &ServiceState<A, V, K>,
    conn: &Connection,
    requests: Vec<StreamRequest>,
) -> Result<BinaryPlist<SetupResponse>, StatusCode> {
    let mut responses = Vec::with_capacity(requests.len());
    for stream in requests {
        let id = state.last_stream_id.fetch_add(1, Ordering::AcqRel);
        match match stream {
            StreamRequest::AudioBuffered(request) => {
                setup_buffered_audio(state, conn, request, id).await
            }
            StreamRequest::AudioRealtime(request) => {
                setup_realtime_audio(state, conn, request, id).await
            }
            StreamRequest::Video(request) => setup_video(state, conn, request, id).await,
        } {
            Ok(response) => responses.push(response),
            Err(err) => return Err(err),
        }
    }

    Ok(BinaryPlist(SetupResponse::Streams { responses }))
}

#[tracing::instrument(level = "DEBUG", ret, err, skip(state))]
async fn setup_buffered_audio<A: AudioDevice, V, K>(
    state: &ServiceState<A, V, K>,
    conn: &Connection,
    AudioBufferedRequest {
        samples_per_frame,
        audio_format,
        audio_format_index,
        shared_key,
        ..
    }: AudioBufferedRequest,
    id: u64,
) -> Result<StreamResponse, StatusCode> {
    let Some(codec) = AUDIO_FORMATS
        .get(audio_format_index.map_or_else(|| audio_format.trailing_zeros() as usize, usize::from))
        .copied()
    else {
        tracing::error!(
            %audio_format,
            ?audio_format_index,
            "unknown audio codec"
        );
        return Err(StatusCode::BAD_REQUEST);
    };
    tracing::debug!(?codec, "codec parsed");

    let key = ChaCha20Poly1305Key::try_from(shared_key.as_ref())
        .inspect_err(|_| {
            tracing::error!(
                len = shared_key.len(),
                "insufficient length of key for buffered audio's decryption"
            );
        })
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let shared_data = Arc::new(SharedData::default());
    let params = AudioParams {
        samples_per_frame,
        codec,
    };
    let stream = state
        .config
        .audio
        .device
        .create(
            id,
            params,
            Arc::downgrade(&shared_data) as Weak<dyn ChannelHandle>,
        )
        .await
        .inspect(|_| tracing::trace!("new stream opened"))
        .inspect_err(|err| tracing::error!(%err, ?params, "stream couldn't be created"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    AudioBufferedChannel::create(
        conn.bind_addr().unwrap(),
        conn.remote_addr().unwrap(),
        shared_data.clone(),
        stream,
        state.config.audio.buf_size,
        key,
    )
    .await
    .inspect(|_| {
        state
            .stream_channels
            .lock()
            .unwrap()
            .insert((id, StreamType::AudioBuffered as u32), shared_data);
    })
    .map(|chan| StreamResponse::AudioBuffered {
        id,
        local_data_port: chan.local_addr.port(),
        audio_buffer_size: chan.audio_buf_size,
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tracing::instrument(level = "DEBUG", ret, err, skip(state))]
async fn setup_realtime_audio<A: AudioDevice, V, K>(
    state: &ServiceState<A, V, K>,
    conn: &Connection,
    AudioRealtimeRequest {
        audio_format,
        samples_per_frame,
        stream_connection_id,
        ..
    }: AudioRealtimeRequest,
    id: u64,
) -> Result<StreamResponse, StatusCode> {
    // This must work like that
    #[allow(clippy::cast_sign_loss)]
    let stream_connection_id = stream_connection_id.map(|x| x as u64);

    let Some(codec) = AUDIO_FORMATS
        .get(audio_format.trailing_zeros() as usize)
        .copied()
    else {
        tracing::error!(%audio_format, "unknown audio codec");
        return Err(StatusCode::BAD_REQUEST);
    };
    tracing::debug!(?codec, "codec parsed");

    let shared_data = Arc::new(SharedData::default());
    let params = AudioParams {
        samples_per_frame,
        codec,
    };
    let stream = state
        .config
        .audio
        .device
        .create(
            id,
            params,
            Arc::downgrade(&shared_data) as Weak<dyn ChannelHandle>,
        )
        .await
        .inspect(|_| tracing::trace!("new stream opened"))
        .inspect_err(|err| tracing::error!(%err, ?params, "stream couldn't be created"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session_key = if let Some(session_key) = conn.session_key.read()
        && session_key.upgrade_channel
    {
        Some(session_key)
    } else {
        None
    };
    AudioRealtimeChannel::create(
        conn.bind_addr().unwrap(),
        conn.remote_addr().unwrap(),
        shared_data.clone(),
        stream,
        state.config.audio.buf_size,
        state.ekey.read(),
        state.eiv.read(),
        session_key,
        stream_connection_id,
    )
    .await
    .inspect(|_| {
        state
            .stream_channels
            .lock()
            .unwrap()
            .insert((id, StreamType::AudioRealtime as u32), shared_data);
    })
    .map(|chan| StreamResponse::AudioRealtime {
        id,
        local_data_port: chan.local_data_addr.port(),
        local_control_port: chan.local_control_addr.port(),
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tracing::instrument(level = "DEBUG", ret, err, skip(state))]
async fn setup_video<A, V: VideoDevice, K>(
    state: &ServiceState<A, V, K>,
    conn: &Connection,
    VideoRequest {
        stream_connection_id,
        ..
    }: VideoRequest,
    id: u64,
) -> Result<StreamResponse, StatusCode> {
    // This must work like that
    #[allow(clippy::cast_sign_loss)]
    let stream_connection_id = stream_connection_id as u64;

    let shared_data = Arc::new(SharedData::default());
    let params = VideoParams {};
    let stream = state
        .config
        .video
        .device
        .create(
            id,
            VideoParams {},
            Arc::downgrade(&shared_data) as Weak<dyn ChannelHandle>,
        )
        .await
        .inspect(|_| tracing::trace!("new stream opened"))
        .inspect_err(|err| tracing::error!(%err, ?params, "stream couldn't be created"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session_key = if let Some(session_key) = conn.session_key.read()
        && session_key.upgrade_channel
    {
        Some(session_key)
    } else {
        None
    };
    VideoChannel::create(
        conn.bind_addr().unwrap(),
        conn.remote_addr().unwrap(),
        shared_data.clone(),
        stream,
        state.config.video.buf_size,
        state.ekey.read(),
        state.eiv.read(),
        session_key,
        stream_connection_id,
    )
    .await
    .inspect(|_| {
        state
            .stream_channels
            .lock()
            .unwrap()
            .insert((id, StreamType::Video as u32), shared_data);
    })
    .map(|chan| StreamResponse::Video {
        id,
        local_data_port: chan.local_addr.port(),
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

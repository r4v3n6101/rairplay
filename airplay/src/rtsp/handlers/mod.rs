use std::{
    net::SocketAddr,
    sync::{Arc, Weak, atomic::Ordering},
};

use crate::{
    crypto::{AesIv128, ChaCha20Poly1305Key, hash_aes_key},
    playback::{
        ChannelHandle,
        audio::{AUDIO_FORMATS, AudioDevice, AudioParams},
        video::{VideoDevice, VideoParams},
    },
    streaming::{
        AudioBufferedChannel, AudioRealtimeChannel, EventChannel, SharedData, VideoChannel,
    },
};

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
    transport::ExtendedAddr,
};

mod fairplay;

#[tracing::instrument(level = "TRACE")]
pub async fn generic(bytes: Bytes) {}

#[tracing::instrument(level = "DEBUG", ret, skip(state))]
pub async fn info<A, V>(State(state): State<Arc<ServiceState<A, V>>>) -> BinaryPlist<InfoResponse> {
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
pub async fn fp_setup<A, V>(
    State(state): State<Arc<ServiceState<A, V>>>,
    body: Bytes,
) -> Result<Vec<u8>, StatusCode> {
    fairplay::decode_buf(&body)
        .inspect(|_| {
            // Magic number somehow. Hate em.
            if body.len() == 164 {
                *state.fp_last_msg.lock().unwrap() = body;
                tracing::trace!("fairplay3 last message is saved");
            }
        })
        .inspect_err(|err| tracing::error!(%err, "failed to decode fairplay"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tracing::instrument(level = "DEBUG", ret, err, skip(state))]
pub async fn get_parameter<A: AudioDevice, V>(
    State(state): State<Arc<ServiceState<A, V>>>,
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
pub async fn teardown<A, V>(
    State(state): State<Arc<ServiceState<A, V>>>,
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
                    .filter(|((_, ty), _)| *ty == req.ty)
                    .for_each(|(_, chan)| chan.close());
                stream_channels.retain(|(_, ty), _| *ty != req.ty);
                tracing::info!(type=%req.ty, "teardown stream");
            }
        }
    } else {
        let num = stream_channels.len();
        stream_channels.iter().for_each(|(_, chan)| chan.close());
        stream_channels.clear();
        tracing::info!(%num, "teardown all streams");
    }
}

pub async fn setup<A: AudioDevice, V: VideoDevice>(
    state: State<Arc<ServiceState<A, V>>>,
    connect_info: ConnectInfo<ExtendedAddr>,
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> Result<BinaryPlist<SetupResponse>, StatusCode> {
    match req {
        SetupRequest::SenderInfo(info) => setup_info(state, connect_info, *info).await,
        SetupRequest::Streams { requests } => setup_streams(state, connect_info, requests).await,
    }
}

#[tracing::instrument(level = "DEBUG", ret, err, skip(state))]
async fn setup_info<A, V>(
    State(state): State<Arc<ServiceState<A, V>>>,
    ConnectInfo(addrs): ConnectInfo<ExtendedAddr>,
    SenderInfo { ekey, eiv, .. }: SenderInfo,
) -> Result<BinaryPlist<SetupResponse>, StatusCode> {
    let mut lock = state.event_channel.lock().await;
    let event_channel = match &mut *lock {
        Some(chan) => chan,
        event_channel @ None => EventChannel::create(SocketAddr::new(addrs.local_addr().ip(), 0))
            .await
            .map(|chan| event_channel.insert(chan))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    };

    let Ok(eiv) = AesIv128::try_from(eiv.as_ref()) else {
        tracing::error!(len=%eiv.len(), "invalid length of passed iv");
        return Err(StatusCode::BAD_REQUEST);
    };

    let Some(session_key) = state.session_key.lock().unwrap().clone() else {
        tracing::error!("must be paired before setup call");
        return Err(StatusCode::FORBIDDEN);
    };

    let fp_last_msg = state.fp_last_msg.lock().unwrap();
    let aes_key = fairplay::decrypt_key(fp_last_msg.as_ref(), &ekey);
    tracing::trace!(
        ?ekey,
        ?aes_key,
        ?fp_last_msg,
        "aes key decrypted with fairplay"
    );

    *state.ekey.lock().unwrap() = hash_aes_key(aes_key, session_key);
    *state.eiv.lock().unwrap() = eiv;

    // TODO : log more info from SenderInfo

    Ok(BinaryPlist(SetupResponse::Info {
        event_port: event_channel.local_addr().port(),
        timing_port: 0,
    }))
}

#[tracing::instrument(level = "DEBUG", skip_all)]
async fn setup_streams<A: AudioDevice, V: VideoDevice>(
    State(state): State<Arc<ServiceState<A, V>>>,
    connect_info: ConnectInfo<ExtendedAddr>,
    requests: Vec<StreamRequest>,
) -> Result<BinaryPlist<SetupResponse>, StatusCode> {
    let mut responses = Vec::with_capacity(requests.len());
    for stream in requests {
        let id = state.last_stream_id.fetch_add(1, Ordering::AcqRel);
        match match stream {
            StreamRequest::AudioRealtime(request) => {
                setup_realtime_audio(&state, connect_info, request, id).await
            }
            StreamRequest::AudioBuffered(request) => {
                setup_buffered_audio(&state, connect_info, request, id).await
            }
            StreamRequest::Video(request) => setup_video(&state, connect_info, request, id).await,
        } {
            Ok(response) => responses.push(response),
            Err(err) => return Err(err),
        }
    }

    Ok(BinaryPlist(SetupResponse::Streams { responses }))
}

#[tracing::instrument(level = "DEBUG", ret, err, skip(state))]
async fn setup_realtime_audio<A: AudioDevice, V>(
    state: &ServiceState<A, V>,
    ConnectInfo(addrs): ConnectInfo<ExtendedAddr>,
    AudioRealtimeRequest {
        audio_format,
        samples_per_frame,
        ..
    }: AudioRealtimeRequest,
    id: u64,
) -> Result<StreamResponse, StatusCode> {
    let Some(codec) = AUDIO_FORMATS
        .get(audio_format.trailing_zeros() as usize)
        .copied()
    else {
        tracing::error!(%audio_format, "unknown audio codec");
        return Err(StatusCode::BAD_REQUEST);
    };
    tracing::debug!(?codec, "codec parsed");

    let key = *state.ekey.lock().unwrap();
    let iv = *state.eiv.lock().unwrap();

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

    AudioRealtimeChannel::create(
        SocketAddr::new(addrs.local_addr().ip(), 0),
        SocketAddr::new(addrs.local_addr().ip(), 0),
        shared_data.clone(),
        stream,
        state.config.audio.buf_size,
        key,
        iv,
    )
    .await
    .inspect(|_| {
        state
            .stream_channels
            .lock()
            .unwrap()
            .insert((id, StreamType::AUDIO_REALTIME), shared_data);
    })
    .map(|chan| StreamResponse::AudioRealtime {
        id,
        local_data_port: chan.local_data_addr.port(),
        local_control_port: chan.local_control_addr.port(),
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tracing::instrument(level = "DEBUG", ret, err, skip(state))]
async fn setup_buffered_audio<A: AudioDevice, V>(
    state: &ServiceState<A, V>,
    ConnectInfo(addrs): ConnectInfo<ExtendedAddr>,
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
        SocketAddr::new(addrs.local_addr().ip(), 0),
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
            .insert((id, StreamType::AUDIO_BUFFERED), shared_data);
    })
    .map(|chan| StreamResponse::AudioBuffered {
        id,
        local_data_port: chan.local_addr.port(),
        audio_buffer_size: chan.audio_buf_size,
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tracing::instrument(level = "DEBUG", ret, err, skip(state))]
async fn setup_video<A, V: VideoDevice>(
    state: &ServiceState<A, V>,
    ConnectInfo(addrs): ConnectInfo<ExtendedAddr>,
    VideoRequest {
        stream_connection_id,
        ..
    }: VideoRequest,
    id: u64,
) -> Result<StreamResponse, StatusCode> {
    // This must work like that
    #[allow(clippy::cast_sign_loss)]
    let stream_connection_id = stream_connection_id as u64;
    let key = *state.ekey.lock().unwrap();

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

    VideoChannel::create(
        SocketAddr::new(addrs.local_addr().ip(), 0),
        shared_data.clone(),
        stream,
        state.config.video.buf_size,
        key,
        stream_connection_id,
    )
    .await
    .inspect(|_| {
        state
            .stream_channels
            .lock()
            .unwrap()
            .insert((id, StreamType::VIDEO), shared_data);
    })
    .map(|chan| StreamResponse::Video {
        id,
        local_data_port: chan.local_addr.port(),
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

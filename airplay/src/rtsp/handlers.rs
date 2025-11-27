use std::{
    net::SocketAddr,
    sync::{Arc, Weak, atomic::Ordering},
};

use crate::{
    crypto::{AesIv128, ChaCha20Poly1305Key, fairplay, hash_aes_key},
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
    extract::State,
    response::{IntoResponse, Response},
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
};

pub async fn generic(bytes: Bytes) {
    tracing::trace!(?bytes, "generic handler");
}

pub async fn info<A, V>(State(state): State<Arc<ServiceState<A, V>>>) -> impl IntoResponse {
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

pub async fn fp_setup<A, V>(
    State(state): State<Arc<ServiceState<A, V>>>,
    body: Bytes,
) -> impl IntoResponse {
    fairplay::decode_buf(&body)
        .inspect(|_| {
            // Magic number somehow. Hate em.
            if body.len() == 164 {
                *state.fp_last_msg.lock().unwrap() = body;
            }
        })
        .inspect_err(|err| tracing::error!(%err, "failed to decode fairplay"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn get_parameter<A: AudioDevice, V>(
    State(state): State<Arc<ServiceState<A, V>>>,
    body: String,
) -> impl IntoResponse {
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
            } else {
                stream_channels
                    .iter()
                    .filter(|((_, ty), _)| *ty == req.ty)
                    .for_each(|(_, chan)| chan.close());
                stream_channels.retain(|(_, ty), _| *ty != req.ty);
            }
        }
    } else {
        stream_channels.iter().for_each(|(_, chan)| chan.close());
        stream_channels.clear();
    }
}

pub async fn setup<A: AudioDevice, V: VideoDevice>(
    state: State<Arc<ServiceState<A, V>>>,
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> impl IntoResponse {
    match req {
        SetupRequest::SenderInfo(info) => setup_info(state, *info).await.into_response(),
        SetupRequest::Streams { requests } => setup_streams(state, requests).await.into_response(),
    }
}

async fn setup_info<A, V>(
    State(state): State<Arc<ServiceState<A, V>>>,
    SenderInfo { ekey, eiv, .. }: SenderInfo,
) -> impl IntoResponse {
    let mut lock = state.event_channel.lock().await;
    let event_channel = match &mut *lock {
        Some(chan) => chan,
        event_channel @ None => EventChannel::create(SocketAddr::new(state.config.bind_addr, 0))
            .await
            .inspect_err(|err| tracing::error!(%err, "failed creating event listener"))
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

    let aes_key = fairplay::decrypt_key(state.fp_last_msg.lock().unwrap().as_ref(), ekey);
    let aes_digest = hash_aes_key(aes_key, session_key);

    *state.ekey.lock().unwrap() = aes_digest;
    *state.eiv.lock().unwrap() = eiv;

    // TODO : log more info from SenderInfo

    Ok(BinaryPlist(SetupResponse::Info {
        event_port: event_channel.local_addr().port(),
        timing_port: 0,
    }))
}

async fn setup_streams<A: AudioDevice, V: VideoDevice>(
    State(state): State<Arc<ServiceState<A, V>>>,
    requests: Vec<StreamRequest>,
) -> Response {
    let mut responses = Vec::with_capacity(requests.len());
    for stream in requests {
        let id = state.last_stream_id.fetch_add(1, Ordering::AcqRel);
        match match stream {
            StreamRequest::AudioRealtime(request) => {
                setup_realtime_audio(&state, request, id).await
            }
            StreamRequest::AudioBuffered(request) => {
                setup_buffered_audio(&state, request, id).await
            }
            StreamRequest::Video(request) => setup_video(&state, request, id).await,
        } {
            Ok(response) => responses.push(response),
            Err(err) => return err,
        }
    }

    BinaryPlist(SetupResponse::Streams { responses }).into_response()
}

async fn setup_realtime_audio<A: AudioDevice, V>(
    state: &ServiceState<A, V>,
    AudioRealtimeRequest {
        audio_format,
        samples_per_frame,
        ..
    }: AudioRealtimeRequest,
    id: u64,
) -> Result<StreamResponse, Response> {
    let Some(codec) = AUDIO_FORMATS
        .get(audio_format.trailing_zeros() as usize)
        .copied()
    else {
        tracing::error!(%audio_format, "unknown audio codec");
        return Err(StatusCode::BAD_REQUEST.into_response());
    };

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
        .inspect_err(|err| tracing::error!(%err, ?params, "stream couldn't be created"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    AudioRealtimeChannel::create(
        SocketAddr::new(state.config.bind_addr, 0),
        SocketAddr::new(state.config.bind_addr, 0),
        state.config.audio.buf_size,
        shared_data.clone(),
        key,
        iv,
        stream,
    )
    .await
    .inspect(|_| {
        state
            .stream_channels
            .lock()
            .unwrap()
            .insert((id, StreamType::AUDIO_REALTIME), shared_data);
    })
    .inspect_err(|err| tracing::error!(%err, "realtime audio listener not created"))
    .map(|chan| StreamResponse::AudioRealtime {
        id,
        local_data_port: chan.local_data_addr.port(),
        local_control_port: chan.local_control_addr.port(),
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn setup_buffered_audio<A: AudioDevice, V>(
    state: &ServiceState<A, V>,
    AudioBufferedRequest {
        samples_per_frame,
        audio_format,
        audio_format_index,
        shared_key,
        ..
    }: AudioBufferedRequest,
    id: u64,
) -> Result<StreamResponse, Response> {
    let Some(codec) = AUDIO_FORMATS
        .get(audio_format_index.map_or_else(|| audio_format.trailing_zeros() as usize, usize::from))
        .copied()
    else {
        tracing::error!(
            %audio_format,
            ?audio_format_index,
            "unknown audio codec"
        );
        return Err(StatusCode::BAD_REQUEST.into_response());
    };

    let key = ChaCha20Poly1305Key::try_from(shared_key.as_ref())
        .inspect_err(|_| {
            tracing::error!(
                len = shared_key.len(),
                "insufficient length of key for buffered audio's decryption"
            );
        })
        .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;

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
        .inspect_err(|err| tracing::error!(%err, ?params, "stream couldn't be created"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    AudioBufferedChannel::create(
        SocketAddr::new(state.config.bind_addr, 0),
        state.config.audio.buf_size,
        shared_data.clone(),
        key,
        stream,
    )
    .await
    .inspect(|_| {
        state
            .stream_channels
            .lock()
            .unwrap()
            .insert((id, StreamType::AUDIO_BUFFERED), shared_data);
    })
    .inspect_err(|err| tracing::error!(%err, "buffered audio listener not created"))
    .map(|chan| StreamResponse::AudioBuffered {
        id,
        local_data_port: chan.local_addr.port(),
        audio_buffer_size: chan.audio_buf_size,
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn setup_video<A, V: VideoDevice>(
    state: &ServiceState<A, V>,
    VideoRequest {
        stream_connection_id,
        ..
    }: VideoRequest,
    id: u64,
) -> Result<StreamResponse, Response> {
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
        .inspect_err(|err| tracing::error!(%err, ?params, "stream couldn't be created"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    VideoChannel::create(
        SocketAddr::new(state.config.bind_addr, 0),
        state.config.video.buf_size,
        shared_data.clone(),
        key,
        stream_connection_id,
        stream,
    )
    .await
    .inspect(|_| {
        state
            .stream_channels
            .lock()
            .unwrap()
            .insert((id, StreamType::VIDEO), shared_data);
    })
    .inspect_err(|err| tracing::error!(%err, "video listener not created"))
    .map(|chan| StreamResponse::Video {
        id,
        local_data_port: chan.local_addr.port(),
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

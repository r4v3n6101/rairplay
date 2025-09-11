use std::{
    net::SocketAddr,
    sync::{atomic::Ordering, Arc, Weak},
};

use crate::{
    crypto::{
        fairplay, hash_aes_key,
        pairing::legacy::{SIGNATURE_LENGTH, X25519_KEY_LEN},
        streaming::{AudioBufferedCipher, AudioRealtimeCipher, VideoCipher},
        AesIv128,
    },
    playback::{
        audio::{AudioDevice, AudioParams, AUDIO_FORMATS},
        video::{VideoDevice, VideoParams},
        ChannelHandle,
    },
    streaming::{
        AudioBufferedChannel, AudioRealtimeChannel, EventChannel, SharedData, VideoChannel,
    },
};

use axum::{
    extract::{ConnectInfo, State},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::{header::CONTENT_TYPE, status::StatusCode};

use super::{
    dto::{
        AudioBufferedRequest, AudioRealtimeRequest, Display, InfoResponse, SenderInfo,
        SetupRequest, SetupResponse, StreamId, StreamRequest, StreamResponse, Teardown,
        VideoRequest,
    },
    extractor::BinaryPlist,
    state::SharedState,
};

pub async fn generic(bytes: Bytes) {
    tracing::trace!(?bytes, "generic handler");
}

pub async fn info<A, V>(State(state): State<SharedState<A, V>>) -> impl IntoResponse {
    const PROTOVERS: &str = "1.1";
    const SRCVERS: &str = "770.8.1";

    let response = InfoResponse {
        device_id: state.cfg.mac_addr,
        mac_addr: state.cfg.mac_addr,
        features: state.cfg.features.bits(),
        protocol_version: PROTOVERS.to_string(),
        source_version: SRCVERS.to_string(),

        manufacturer: state.cfg.manufacturer.clone(),
        model: state.cfg.model.clone(),
        name: state.cfg.name.clone(),

        // Seems like clients don't respect other displays and pick by maximum resolution
        displays: vec![Display {
            width: state.cfg.video.width,
            height: state.cfg.video.height,
            uuid: format!("{}_display", state.cfg.name),
            max_fps: state.cfg.video.fps,
            features: 2,
        }],
    };

    BinaryPlist(response)
}

/// Don't really need request body here, because it duplicates signing key of counterparty got in
/// the second request.
pub async fn pair_setup<A, V>(State(state): State<SharedState<A, V>>) -> impl IntoResponse {
    state.pairing.lock().unwrap().verifying_key()
}

pub async fn pair_verify<A, V>(
    State(state): State<SharedState<A, V>>,
    body: Bytes,
) -> impl IntoResponse {
    if body.len() < 4 + 2 * X25519_KEY_LEN {
        tracing::error!(len=%body.len(), "malformed data for legacy pairing");
        return Err(StatusCode::BAD_REQUEST);
    }

    let mode = body[0];
    if mode > 0 {
        let pubkey_their = body[4..][..X25519_KEY_LEN].try_into().unwrap();
        let verify_their = body[36..][..X25519_KEY_LEN].try_into().unwrap();

        state
            .pairing
            .lock()
            .unwrap()
            .establish_agreement(pubkey_their, verify_their)
            .inspect_err(|err| tracing::error!(%err, "legacy pairing establishment failed"))
            .map(IntoResponse::into_response)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    } else {
        let signature = body[4..][..SIGNATURE_LENGTH].try_into().unwrap();

        state
            .pairing
            .lock()
            .unwrap()
            .verify_agreement(signature)
            .inspect_err(|err| tracing::error!(%err, "legacy pairing verification failed"))
            .map(IntoResponse::into_response)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub async fn fp_setup<A, V>(
    State(state): State<SharedState<A, V>>,
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
    State(state): State<SharedState<A, V>>,
    body: String,
) -> impl IntoResponse {
    match body.as_str() {
        "volume\r\n" => {
            let volume = state.cfg.audio.device.get_volume();
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
    State(state): State<SharedState<A, V>>,
    BinaryPlist(req): BinaryPlist<Teardown>,
) {
    let mut audio_realtime_channels = state.audio_realtime_channels.lock().unwrap();
    let mut audio_buffered_channels = state.audio_buffered_channels.lock().unwrap();
    let mut video_channels = state.video_channels.lock().unwrap();
    if let Some(requests) = req.requests {
        for req in requests {
            if let Some(id) = req.id {
                if let Some(chan) = audio_realtime_channels.remove(&id) {
                    chan.close();
                }
                if let Some(chan) = audio_buffered_channels.remove(&id) {
                    chan.close();
                }
                if let Some(chan) = video_channels.remove(&id) {
                    chan.close();
                }
            } else {
                match req.ty {
                    StreamId::AUDIO_REALTIME => {
                        audio_realtime_channels.drain().for_each(|(_, c)| c.close());
                    }
                    StreamId::AUDIO_BUFFERED => {
                        audio_buffered_channels.drain().for_each(|(_, c)| c.close());
                    }
                    StreamId::VIDEO => video_channels.drain().for_each(|(_, c)| c.close()),
                    _ => {}
                }
            }
        }
    } else {
        audio_realtime_channels.drain().for_each(|(_, c)| c.close());
        audio_buffered_channels.drain().for_each(|(_, c)| c.close());
        video_channels.drain().for_each(|(_, c)| c.close());
    }
}

pub async fn setup<A: AudioDevice, V: VideoDevice>(
    state: State<SharedState<A, V>>,
    connect_info: ConnectInfo<SocketAddr>,
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> impl IntoResponse {
    match req {
        SetupRequest::SenderInfo(info) => {
            setup_info(state, connect_info, *info).await.into_response()
        }
        SetupRequest::Streams { requests } => setup_streams(state, connect_info, requests)
            .await
            .into_response(),
    }
}

async fn setup_info<A, V>(
    State(state): State<SharedState<A, V>>,
    ConnectInfo(local_addr): ConnectInfo<SocketAddr>,
    SenderInfo { ekey, eiv, .. }: SenderInfo,
) -> impl IntoResponse {
    let mut lock = state.event_channel.lock().await;
    let event_channel = match &mut *lock {
        Some(chan) => chan,
        event_channel @ None => EventChannel::create(SocketAddr::new(local_addr.ip(), 0))
            .await
            .inspect_err(|err| tracing::error!(%err, "failed creating event listener"))
            .map(|chan| event_channel.insert(chan))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    };

    let Ok(eiv) = AesIv128::try_from(eiv.as_ref()) else {
        tracing::error!(len=%eiv.len(), "invalid length of passed iv");
        return Err(StatusCode::BAD_REQUEST);
    };

    let Some(shared_secret) = state.pairing.lock().unwrap().shared_secret() else {
        tracing::error!("must be paired before setup call");
        return Err(StatusCode::FORBIDDEN);
    };

    let aes_key = fairplay::decrypt_key(state.fp_last_msg.lock().unwrap().as_ref(), ekey);
    let aes_digest = hash_aes_key(aes_key, shared_secret);

    *state.ekey.lock().unwrap() = aes_digest;
    *state.eiv.lock().unwrap() = eiv;

    // TODO : log more info from SenderInfo

    Ok(BinaryPlist(SetupResponse::Info {
        event_port: event_channel.local_addr().port(),
        timing_port: 0,
    }))
}

async fn setup_streams<A: AudioDevice, V: VideoDevice>(
    State(state): State<SharedState<A, V>>,
    ConnectInfo(local_addr): ConnectInfo<SocketAddr>,
    requests: Vec<StreamRequest>,
) -> Response {
    let mut responses = Vec::with_capacity(requests.len());
    for stream in requests {
        let id = state.last_stream_id.fetch_add(1, Ordering::AcqRel);
        match match stream {
            StreamRequest::AudioRealtime(request) => {
                setup_realtime_audio(state.clone(), local_addr, request, id).await
            }
            StreamRequest::AudioBuffered(request) => {
                setup_buffered_audio(state.clone(), local_addr, request, id).await
            }
            StreamRequest::Video(request) => {
                setup_video(state.clone(), local_addr, request, id).await
            }
        } {
            Ok(response) => responses.push(response),
            Err(err) => return err,
        }
    }

    BinaryPlist(SetupResponse::Streams { responses }).into_response()
}

async fn setup_realtime_audio<A: AudioDevice, V>(
    state: SharedState<A, V>,
    local_addr: SocketAddr,
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

    let cipher = AudioRealtimeCipher::new(*state.ekey.lock().unwrap(), *state.eiv.lock().unwrap());

    let shared_data = Arc::new(SharedData::default());
    let params = AudioParams {
        samples_per_frame,
        codec,
    };
    let stream = state
        .cfg
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
        SocketAddr::new(local_addr.ip(), 0),
        SocketAddr::new(local_addr.ip(), 0),
        state.cfg.audio.buf_size,
        shared_data.clone(),
        cipher,
        stream,
    )
    .await
    .inspect(|_| {
        state
            .audio_realtime_channels
            .lock()
            .unwrap()
            .insert(id, shared_data);
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
    state: SharedState<A, V>,
    local_addr: SocketAddr,
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

    let cipher = AudioBufferedCipher::new(
        <[u8; AudioBufferedCipher::KEY_LEN]>::try_from(shared_key.as_ref())
            .inspect_err(|_| {
                tracing::error!(
                    len = shared_key.len(),
                    "insufficient length of key for buffered audio's decryption"
                );
            })
            .map_err(|_| StatusCode::BAD_REQUEST.into_response())?,
    );

    let shared_data = Arc::new(SharedData::default());
    let params = AudioParams {
        samples_per_frame,
        codec,
    };
    let stream = state
        .cfg
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
        SocketAddr::new(local_addr.ip(), 0),
        state.cfg.audio.buf_size,
        shared_data.clone(),
        cipher,
        stream,
    )
    .await
    .inspect(|_| {
        state
            .audio_buffered_channels
            .lock()
            .unwrap()
            .insert(id, shared_data);
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
    state: SharedState<A, V>,
    local_addr: SocketAddr,
    VideoRequest {
        stream_connection_id,
        ..
    }: VideoRequest,
    id: u64,
) -> Result<StreamResponse, Response> {
    // This must work like that
    #[allow(clippy::cast_sign_loss)]
    let cipher = VideoCipher::new(*state.ekey.lock().unwrap(), stream_connection_id as u64);

    let shared_data = Arc::new(SharedData::default());
    let params = VideoParams {};
    let stream = state
        .cfg
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
        SocketAddr::new(local_addr.ip(), 0),
        state.cfg.video.buf_size,
        shared_data.clone(),
        cipher,
        stream,
    )
    .await
    .inspect(|_| {
        state.video_channels.lock().unwrap().insert(id, shared_data);
    })
    .inspect_err(|err| tracing::error!(%err, "video listener not created"))
    .map(|chan| StreamResponse::Video {
        id,
        local_data_port: chan.local_addr.port(),
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

use std::{
    net::SocketAddr,
    sync::{atomic::Ordering, Arc, Weak},
    time::Duration,
};

use crate::{
    crypto::{
        fairplay,
        pairing::legacy::{SIGNATURE_LENGTH, X25519_KEY_LEN},
    },
    device::{AudioDevice, AudioParams, ChannelHandle, VideoDevice, VideoParams},
    streaming::{self, audio, event::Channel as EventChannel, video},
    util::constants,
};

use axum::{
    extract::{ConnectInfo, State},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::{header::CONTENT_TYPE, status::StatusCode};

use super::{
    dto::{
        AudioBufferedRequest, AudioRealtimeRequest, Display, FlushBufferedRequest, InfoResponse,
        SenderInfo, SetRateAnchorTimeRequest, SetupRequest, SetupResponse, StreamId, StreamRequest,
        StreamResponse, Teardown, VideoRequest,
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
        let pubkey_their = body[4..][..X25519_KEY_LEN]
            .try_into()
            .expect("x25519 key must be 32 bytes");
        let verify_their = body[36..][..X25519_KEY_LEN]
            .try_into()
            .expect("ed25519 key must be 32 bytes");

        state
            .pairing
            .lock()
            .unwrap()
            .establish_agreement(pubkey_their, verify_their)
            .inspect_err(|err| tracing::error!(%err, "legacy pairing establishment failed"))
            .map(IntoResponse::into_response)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    } else {
        let signature = body[4..][..SIGNATURE_LENGTH]
            .try_into()
            .expect("signature must be 64 bytes");

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
    fairplay::decode_buf(body.clone())
        .inspect(|_| {
            // Magic number somehow. Hate em.
            if body.len() == 164 {
                *state.fp_last_msg.lock().unwrap() = body.clone();
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

pub async fn set_parameter(body: Bytes) {}

pub async fn flush() {}

pub async fn flush_buffered(BinaryPlist(req): BinaryPlist<FlushBufferedRequest>) {}

pub async fn set_rate_anchor_time(BinaryPlist(req): BinaryPlist<SetRateAnchorTimeRequest>) {}

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
        SetupRequest::SenderInfo { info } => {
            setup_info(state, connect_info, info).await.into_response()
        }
        SetupRequest::Streams { requests } => setup_streams(state, connect_info, requests)
            .await
            .into_response(),
    }
}

async fn setup_info<A, V>(
    State(state): State<SharedState<A, V>>,
    ConnectInfo(local_addr): ConnectInfo<SocketAddr>,
    SenderInfo { ekey, .. }: SenderInfo,
) -> impl IntoResponse {
    let mut lock = state.event_channel.lock().await;
    let event_channel = match &mut *lock {
        Some(chan) => chan,
        event_channel @ None => {
            match EventChannel::create(SocketAddr::new(local_addr.ip(), 0)).await {
                Ok(chan) => event_channel.insert(chan),
                Err(err) => {
                    tracing::error!(%err, "failed creating event listener");
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
    };

    // TODO : log more info from SenderInfo

    *state.fp_key.lock().unwrap() = Bytes::from_owner(fairplay::decrypt_key(
        state.fp_last_msg.lock().unwrap().as_ref(),
        ekey,
    ));

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
            StreamRequest::AudioRealtime { request } => {
                setup_realtime_audio(state.clone(), local_addr, request, id).await
            }
            StreamRequest::AudioBuffered { request } => {
                setup_buffered_audio(state.clone(), local_addr, request, id).await
            }
            StreamRequest::Video { request } => {
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
    let Some(codec) = constants::AUDIO_FORMATS
        .get(audio_format.trailing_zeros() as usize)
        .copied()
    else {
        tracing::error!(%audio_format, "unknown audio codec");
        return Err(StatusCode::BAD_REQUEST.into_response());
    };

    let shared_data = Arc::new(audio::RealtimeSharedData::default());
    let stream = state.cfg.audio.device.create(
        AudioParams {
            samples_per_frame,
            codec,
        },
        Arc::downgrade(&shared_data) as Weak<dyn ChannelHandle>,
    );
    match streaming::audio::RealtimeChannel::create(
        SocketAddr::new(local_addr.ip(), 0),
        SocketAddr::new(local_addr.ip(), 0),
        state.cfg.audio.buf_size,
        shared_data.clone(),
        stream,
    )
    .await
    {
        Ok(chan) => {
            let local_data_port = chan.local_data_addr.port();
            let local_control_port = chan.local_control_addr.port();
            state
                .audio_realtime_channels
                .lock()
                .unwrap()
                .insert(id, shared_data);

            Ok(StreamResponse::AudioRealtime {
                id,
                local_data_port,
                local_control_port,
            })
        }
        Err(err) => {
            tracing::error!(%err, "realtime audio listener not created");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

async fn setup_buffered_audio<A: AudioDevice, V>(
    state: SharedState<A, V>,
    local_addr: SocketAddr,
    AudioBufferedRequest {
        samples_per_frame,
        audio_format,
        audio_format_index,
        ..
    }: AudioBufferedRequest,
    id: u64,
) -> Result<StreamResponse, Response> {
    let Some(codec) = constants::AUDIO_FORMATS
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

    let shared_data = Arc::new(audio::BufferedSharedData::default());
    let stream = state.cfg.audio.device.create(
        AudioParams {
            samples_per_frame,
            codec,
        },
        Arc::downgrade(&shared_data) as Weak<dyn ChannelHandle>,
    );

    match streaming::audio::BufferedChannel::create(
        SocketAddr::new(local_addr.ip(), 0),
        state.cfg.audio.buf_size,
        shared_data.clone(),
        stream,
    )
    .await
    {
        Ok(chan) => {
            let local_data_port = chan.local_addr.port();
            let audio_buffer_size = chan.audio_buf_size;
            state
                .audio_buffered_channels
                .lock()
                .unwrap()
                .insert(id, shared_data);

            Ok(StreamResponse::AudioBuffered {
                id,
                local_data_port,
                audio_buffer_size,
            })
        }
        Err(err) => {
            tracing::error!(%err, "buffered audio listener not created");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

async fn setup_video<A, V: VideoDevice>(
    state: SharedState<A, V>,
    local_addr: SocketAddr,
    VideoRequest { latency_ms, .. }: VideoRequest,
    id: u64,
) -> Result<StreamResponse, Response> {
    // let cipher = state.pairing.lock().unwrap().shared_secret().map(
    //     |shared_secret| {
    //         VideoCipher::new(
    //             state.fp_key.lock().unwrap().as_ref(),
    //             shared_secret,
    //             stream_connection_id,
    //         )
    //     },
    // );

    let shared_data = Arc::new(video::SharedData::default());
    let stream = state.cfg.video.device.create(
        VideoParams {},
        Arc::downgrade(&shared_data) as Weak<dyn ChannelHandle>,
    );

    match streaming::video::Channel::create(
        SocketAddr::new(local_addr.ip(), 0),
        state.cfg.video.buf_size,
        Duration::from_millis(latency_ms.into()),
        shared_data.clone(),
        stream,
    )
    .await
    {
        Ok(chan) => {
            let local_data_port = chan.local_addr.port();

            state.video_channels.lock().unwrap().insert(id, shared_data);

            Ok(StreamResponse::Video {
                id,
                local_data_port,
            })
        }
        Err(err) => {
            tracing::error!(%err, "video listener not created");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

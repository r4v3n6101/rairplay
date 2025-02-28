use std::{net::SocketAddr, sync::atomic::Ordering, time::Duration};

use crate::{
    streaming::{
        audio::{
            buffered::Channel as BufferedAudioChannel, realtime::Channel as RealtimeAudioChannel,
        },
        event::Channel as EventChannel,
        video::Channel as VideoChannel,
    },
    util::crypto::{
        fairplay,
        pairing::legacy::{SIGNATURE_LENGTH, X25519_KEY_LEN},
        video::Cipher as VideoCipher,
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
        Display, InfoResponse, SetRateAnchorTimeRequest, SetupRequest, SetupResponse,
        StreamDescriptor, StreamRequest,
    },
    extractor::BinaryPlist,
    state::SharedState,
};

pub async fn generic(bytes: Option<Bytes>) {
    tracing::trace!(?bytes, "generic handler");
}

pub async fn info(State(state): State<SharedState>) -> impl IntoResponse {
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

        initial_volume: state.cfg.initial_volume,

        // TODO : for testing video
        displays: vec![Display {
            width: 1920,
            height: 1080,
            uuid: "duck-you".to_string(),
            max_fps: 60,
            features: 2,
        }],
    };

    BinaryPlist(response)
}

/// Don't really need request body here, because it duplicates signing key of counterparty got in
/// the second request.
pub async fn pair_setup(State(state): State<SharedState>) -> impl IntoResponse {
    state.pairing.lock().unwrap().verifying_key()
}

pub async fn pair_verify(State(state): State<SharedState>, body: Bytes) -> impl IntoResponse {
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

pub async fn fp_setup(State(state): State<SharedState>, body: Bytes) -> impl IntoResponse {
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

pub async fn get_parameter(State(state): State<SharedState>, body: String) -> impl IntoResponse {
    match body.as_str() {
        "volume\r\n" => {
            let volume = state.cfg.initial_volume.unwrap_or_default();
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

pub async fn set_rate_anchor_time(
    State(state): State<SharedState>,
    BinaryPlist(req): BinaryPlist<SetRateAnchorTimeRequest>,
) {
}

pub async fn flush_buffered(
    State(state): State<SharedState>,
    // BinaryPlist(req): BinaryPlist<FlushBufferedRequest>,
) {
    state.audio_streams.values().for_each(|s| s.flush());
    state.video_streams.values().for_each(|s| s.flush());
}

pub async fn setup(
    State(state): State<SharedState>,
    ConnectInfo(local_addr): ConnectInfo<SocketAddr>,
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> impl IntoResponse {
    match req {
        SetupRequest::SenderInfo { ekey, .. } => {
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

            Ok(BinaryPlist(SetupResponse::General {
                event_port: event_channel.local_addr().port(),
                timing_port: 0,
            }))
        }

        SetupRequest::Streams { requests } => {
            // TODO : move out these defaults into the config
            const MIN_BUF_DEPTH: Duration = Duration::from_millis(20);
            const MAX_BUF_DEPTH: Duration = Duration::from_millis(200);
            const AUDIO_BUF_SIZE: usize = 8 * 1024 * 1024; // 8mb

            let mut descriptors = Vec::with_capacity(requests.len());
            for stream in requests {
                let id = state.last_stream_id.fetch_add(1, Ordering::AcqRel);
                let descriptor =
                    match stream {
                        StreamRequest::AudioBuffered { .. } => {
                            match BufferedAudioChannel::create(
                                SocketAddr::new(local_addr.ip(), 0),
                                AUDIO_BUF_SIZE,
                            )
                            .await
                            {
                                Ok(chan) => StreamDescriptor::AudioBuffered {
                                    id,
                                    local_data_port: chan.local_addr().port(),
                                    audio_buffer_size: chan.audio_buf_size() as u32,
                                },
                                Err(err) => {
                                    tracing::error!(%err, "buffered audio listener not created");
                                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                                }
                            }
                        }

                        StreamRequest::AudioRealtime { .. } => {
                            match RealtimeAudioChannel::create(
                                SocketAddr::new(local_addr.ip(), 0),
                                SocketAddr::new(local_addr.ip(), 0),
                                AUDIO_BUF_SIZE,
                                MIN_BUF_DEPTH,
                                MAX_BUF_DEPTH,
                            )
                            .await
                            {
                                Ok(chan) => StreamDescriptor::AudioRealtime {
                                    id,
                                    local_data_port: chan.local_data_addr().port(),
                                    local_control_port: chan.local_control_addr().port(),
                                },
                                Err(err) => {
                                    tracing::error!(%err, "realtime audio listener not created");
                                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                                }
                            }
                        }

                        StreamRequest::Video {
                            stream_connection_id,
                            ..
                        } => {
                            let cipher = state.pairing.lock().unwrap().shared_secret().map(
                                |shared_secret| {
                                    VideoCipher::new(
                                        state.fp_key.lock().unwrap().as_ref(),
                                        shared_secret,
                                        stream_connection_id,
                                    )
                                },
                            );

                            match VideoChannel::create(SocketAddr::new(local_addr.ip(), 0), cipher)
                                .await
                            {
                                Ok(chan) => StreamDescriptor::Video {
                                    id,
                                    local_data_port: chan.local_addr().port(),
                                },
                                Err(err) => {
                                    tracing::error!(%err, "video listener not created");
                                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                                }
                            }
                        }
                    };

                descriptors.push(descriptor);
            }

            Ok(BinaryPlist(SetupResponse::Streams { descriptors }))
        }
    }
}

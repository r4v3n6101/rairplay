use std::{net::SocketAddr, sync::atomic::Ordering};

use crate::{
    crypto::{
        fairplay,
        pairing::legacy::{SIGNATURE_LENGTH, X25519_KEY_LEN},
    },
    device::{AudioParams, VideoParams},
    streaming::{
        audio::{
            buffered::Channel as BufferedAudioChannel, realtime::Channel as RealtimeAudioChannel,
        },
        event::Channel as EventChannel,
        video::Channel as VideoChannel,
    },
    util::constants,
};

use axum::{
    extract::{ConnectInfo, State},
    response::IntoResponse,
};
use bytes::Bytes;
use http::{header::CONTENT_TYPE, status::StatusCode};

use super::{
    dto::{
        Display, FlushBufferedRequest, InfoResponse, SetRateAnchorTimeRequest, SetupRequest,
        SetupResponse, StreamId, StreamRequest, StreamResponse, Teardown,
    },
    extractor::BinaryPlist,
    state::{SharedState, StreamDescriptor},
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

pub async fn flush_buffered(
    State(state): State<SharedState>,
    BinaryPlist(req): BinaryPlist<FlushBufferedRequest>,
) {
}

pub async fn set_rate_anchor_time(
    State(state): State<SharedState>,
    BinaryPlist(req): BinaryPlist<SetRateAnchorTimeRequest>,
) {
}

pub async fn teardown(State(state): State<SharedState>, BinaryPlist(req): BinaryPlist<Teardown>) {
    let mut handles = state.stream_handles.lock().unwrap();
    if let Some(requests) = req.requests {
        for req in requests {
            if let Some(id) = req.id {
                tracing::info!(%id, "cleaning up stream by id");
                handles.retain(|k, _| k.id != id);
            } else {
                tracing::info!(type=%req.ty, "cleaning up stream(s) by type");
                handles.retain(|k, _| k.ty != req.ty);
            }
        }
    } else {
        tracing::info!(remaining=%handles.len(), "cleaning up all streams");
        handles.clear();
    }
}

// TODO : split the method into 2-s
#[allow(clippy::too_many_lines)]
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
            let mut descriptors = Vec::with_capacity(requests.len());
            for stream in requests {
                let id = state.last_stream_id.fetch_add(1, Ordering::AcqRel);
                let descriptor = match stream {
                    StreamRequest::AudioBuffered {
                        samples_per_frame,
                        audio_format,
                        audio_format_index,
                        ..
                    } => {
                        let Some(codec) = constants::AUDIO_FORMATS
                            .get(audio_format_index.map_or_else(
                                || audio_format.trailing_zeros() as usize,
                                usize::from,
                            ))
                            .copied()
                        else {
                            tracing::error!(
                                %audio_format,
                                ?audio_format_index,
                                "unknown audio codec"
                            );
                            return Err(StatusCode::BAD_REQUEST);
                        };

                        match BufferedAudioChannel::create(
                            SocketAddr::new(local_addr.ip(), 0),
                            state.cfg.audio.audio_buf_size,
                        )
                        .await
                        {
                            Ok(chan) => {
                                let stream = state.cfg.audio.device.create(
                                    AudioParams {
                                        samples_per_frame,
                                        codec,
                                    },
                                    chan.data_callback(),
                                );
                                state.stream_handles.lock().unwrap().insert(
                                    StreamDescriptor {
                                        id,
                                        ty: StreamId::AudioBuffered as u32,
                                    },
                                    stream,
                                );

                                StreamResponse::AudioBuffered {
                                    id,
                                    local_data_port: chan.local_addr().port(),
                                    audio_buffer_size: chan.audio_buf_size(),
                                }
                            }
                            Err(err) => {
                                tracing::error!(%err, "buffered audio listener not created");
                                return Err(StatusCode::INTERNAL_SERVER_ERROR);
                            }
                        }
                    }

                    StreamRequest::AudioRealtime {
                        sample_rate,
                        samples_per_frame,
                        audio_format,
                        ..
                    } => {
                        let Some(codec) = constants::AUDIO_FORMATS
                            .get(audio_format.trailing_zeros() as usize)
                            .copied()
                        else {
                            tracing::error!(%audio_format, "unknown audio codec");
                            return Err(StatusCode::BAD_REQUEST);
                        };

                        match RealtimeAudioChannel::create(
                            SocketAddr::new(local_addr.ip(), 0),
                            SocketAddr::new(local_addr.ip(), 0),
                            state.cfg.audio.audio_buf_size,
                            state.cfg.audio.min_jitter_depth,
                            state.cfg.audio.max_jitter_depth,
                            sample_rate,
                        )
                        .await
                        {
                            Ok(chan) => {
                                let stream = state.cfg.audio.device.create(
                                    AudioParams {
                                        samples_per_frame,
                                        codec,
                                    },
                                    chan.data_callback(),
                                );
                                state.stream_handles.lock().unwrap().insert(
                                    StreamDescriptor {
                                        id,
                                        ty: StreamId::AudioRealtime as u32,
                                    },
                                    stream,
                                );

                                StreamResponse::AudioRealtime {
                                    id,
                                    local_data_port: chan.local_data_addr().port(),
                                    local_control_port: chan.local_control_addr().port(),
                                }
                            }
                            Err(err) => {
                                tracing::error!(%err, "realtime audio listener not created");
                                return Err(StatusCode::INTERNAL_SERVER_ERROR);
                            }
                        }
                    }

                    StreamRequest::Video {
                        stream_connection_id,
                        latency_ms,
                    } => {
                        // let cipher = state.pairing.lock().unwrap().shared_secret().map(
                        //     |shared_secret| {
                        //         VideoCipher::new(
                        //             state.fp_key.lock().unwrap().as_ref(),
                        //             shared_secret,
                        //             stream_connection_id,
                        //         )
                        //     },
                        // );

                        match VideoChannel::create(SocketAddr::new(local_addr.ip(), 0)).await {
                            Ok(chan) => {
                                let params = VideoParams {};
                                let stream =
                                    state.cfg.video.device.create(params, chan.data_callback());
                                state.stream_handles.lock().unwrap().insert(
                                    StreamDescriptor {
                                        id,
                                        ty: StreamId::Video as u32,
                                    },
                                    stream,
                                );

                                StreamResponse::Video {
                                    id,
                                    local_data_port: chan.local_addr().port(),
                                }
                            }
                            Err(err) => {
                                tracing::error!(%err, "video listener not created");
                                return Err(StatusCode::INTERNAL_SERVER_ERROR);
                            }
                        }
                    }
                };

                descriptors.push(descriptor);
            }

            Ok(BinaryPlist(SetupResponse::Streams {
                response: descriptors,
            }))
        }
    }
}

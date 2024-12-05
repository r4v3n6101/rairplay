use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, State},
    response::IntoResponse,
};
use bytes::Bytes;
use http::{header::CONTENT_TYPE, status::StatusCode};

use crate::streaming;

use super::{
    dto::{Display, InfoResponse, SetupRequest, SetupResponse, StreamDescriptor, StreamRequest},
    plist::BinaryPlist,
    state::SharedState,
};

mod fairplay;

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

pub async fn fp_setup(body: Bytes) -> impl IntoResponse {
    fairplay::decode_buf(body).map_err(|err| {
        tracing::error!(%err, "failed to decode fairplay");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

pub async fn get_parameter(body: String) -> impl IntoResponse {
    match body.as_str() {
        "volume\r\n" => Ok(([(CONTENT_TYPE, "text/parameters")], "volume: 0.0\r\n")),
        param => {
            tracing::error!(?param, "unimplemented parameter");
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    }
}

pub async fn setup(
    State(state): State<SharedState>,
    ConnectInfo(local_addr): ConnectInfo<SocketAddr>,
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> impl IntoResponse {
    match req {
        SetupRequest::SenderInfo { .. } => {
            let mut lock = state.event_channel.lock().await;
            let event_channel = match &mut *lock {
                Some(chan) => chan,
                event_channel @ None => {
                    match streaming::event::Channel::create(SocketAddr::new(local_addr.ip(), 0))
                        .await
                    {
                        Ok(chan) => event_channel.insert(chan),
                        Err(err) => {
                            tracing::error!(%err, "failed creating event listener");
                            return Err(StatusCode::INTERNAL_SERVER_ERROR);
                        }
                    }
                }
            };

            Ok(BinaryPlist(SetupResponse::General {
                event_port: event_channel.local_addr().port(),
                timing_port: 0,
            }))
        }

        SetupRequest::Streams { requests } => {
            let mut descriptors = Vec::with_capacity(requests.len());
            for stream in requests {
                let (dispatcher, handler) = streaming::command::channel();
                let descriptor = match stream {
                    StreamRequest::AudioBuffered { .. } => {
                        // TODO : pass it into config
                        const AUDIO_BUF_SIZE: usize = 8 * 1024 * 1024; // 8mb

                        match streaming::audio::buffered::Channel::create(
                            SocketAddr::new(local_addr.ip(), 0),
                            AUDIO_BUF_SIZE,
                            handler,
                        )
                        .await
                        {
                            Ok(chan) => StreamDescriptor::AudioBuffered {
                                // TODO : id system to get new id's
                                id: 1,
                                local_data_port: chan.local_addr().port(),
                                audio_buffer_size: chan.audio_buf_size() as u32,
                            },
                            Err(err) => {
                                tracing::error!(%err, "buffered audio listener not created");
                                return Err(StatusCode::INTERNAL_SERVER_ERROR);
                            }
                        }
                    }
                    StreamRequest::AudioRealtime { .. } => StreamDescriptor::AudioRealtime {
                        id: 1,
                        local_data_port: 10123,
                        local_control_port: 10124,
                    },
                    StreamRequest::Video { .. } => StreamDescriptor::Video {
                        id: 2,
                        local_data_port: 10125,
                    },
                };

                state.dispatchers.lock().unwrap().push(dispatcher);
                descriptors.push(descriptor);
            }

            Ok(BinaryPlist(SetupResponse::Streams { descriptors }))
        }
    }
}

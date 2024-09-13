use std::net::SocketAddr;

use axum::{
    extract::ConnectInfo,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use hyper::{header::CONTENT_TYPE, StatusCode};

use crate::{adv::Advertisment, streaming};

use super::{
    dto::{
        Display, FlushBufferedRequest, InfoResponse, SetRateAnchorTimeRequest, SetupRequest,
        SetupResponse, StreamDescriptor, StreamRequest,
    },
    fairplay,
    plist::BinaryPlist,
    transport::IncomingStream,
};

pub async fn generic(bytes: Option<Bytes>) {
    tracing::trace!(?bytes, "generic handler");
}

pub async fn info() -> impl IntoResponse {
    const PROTOVERS: &str = "1.1";
    const SRCVERS: &str = "770.8.1";

    let adv = Advertisment::default();
    let response = InfoResponse {
        device_id: adv.mac_addr.to_string(),
        mac_addr: adv.mac_addr.to_string(),
        features: adv.features.bits(),
        protocol_version: PROTOVERS.to_string(),
        source_version: SRCVERS.to_string(),

        manufacturer: adv.manufacturer.clone(),
        model: adv.model.clone(),
        name: adv.name.clone(),

        initial_volume: Some(0.0),

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
        err.to_string()
    })
}

pub async fn get_parameter(body: String) -> Result<Response, StatusCode> {
    match body.as_str() {
        "volume\r\n" => {
            Ok(([(CONTENT_TYPE, "text/parameters")], "volume: 0.0\r\n").into_response())
        }
        param => {
            tracing::error!(?param, "unimplemented parameter");
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    }
}

pub async fn set_rate_anchor_time(request: BinaryPlist<SetRateAnchorTimeRequest>) {
    tracing::debug!(?request);
}

pub async fn flush_buffered(obj: BinaryPlist<FlushBufferedRequest>) {
    tracing::debug!(?obj, "FLUSHBUFFERED");
}

// TODO : stop leaking channels, instead of that store them in axum's state
pub async fn setup(
    ConnectInfo(IncomingStream { local_addr, .. }): ConnectInfo<IncomingStream>,
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> Result<BinaryPlist<SetupResponse>, StatusCode> {
    match req {
        freq @ SetupRequest::SenderInfo { .. } => {
            tracing::info!(?freq, "setup sender's info");

            // TODO : this must be handled better
            let event_channel = streaming::event::spawn_tracing(SocketAddr::new(local_addr, 0))
                .await
                .unwrap();
            let event_port = event_channel.local_addr().port();
            std::mem::forget(event_channel);

            Ok(BinaryPlist(SetupResponse::General {
                event_port,
                timing_port: 0,
            }))
        }

        SetupRequest::Streams { requests } => {
            let mut descriptors = Vec::with_capacity(requests.len());
            for stream in requests {
                let descriptor = match stream {
                    StreamRequest::AudioBuffered { .. } => {
                        let data_channel = streaming::audio::buffered::spawn_processor(
                            SocketAddr::new(local_addr, 9991),
                        )
                        .await
                        .unwrap();

                        let data_port = data_channel.local_addr().port();

                        handles.push(data_channel);
                        StreamDescriptor::AudioBuffered {
                            id: 1,
                            local_data_port: data_port,
                            audio_buffer_size: 8192 * 1024,
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
                descriptors.push(descriptor);
            }

            Ok(BinaryPlist(SetupResponse::Streams { descriptors }))
        }
    }
}

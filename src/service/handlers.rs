use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use hyper::{header::CONTENT_TYPE, StatusCode};

use crate::adv::Advertisment;

use super::{
    dto::{
        Display, FlushBufferedRequest, InfoResponse, SetRateAnchorTimeRequest, SetupRequest,
        SetupResponse,
    },
    fairplay,
    plist::BinaryPlist,
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
        initial_volume: Some(0.0),
        features: adv.features.bits(),
        protocol_version: PROTOVERS.to_string(),
        source_version: SRCVERS.to_string(),

        manufacturer: adv.manufacturer.clone(),
        model: adv.model.clone(),
        name: adv.name.clone(),

        displays: vec![Display {
            width: 1920,
            height: 1080,
            uuid: "duck-you".to_string(),
            max_fps: 60,
            features: 2,
        }],
    };
    tracing::info!(?response, ?adv, "built info from advertisment");

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

pub async fn setup(
    BinaryPlist(req): BinaryPlist<SetupRequest>,
) -> Result<BinaryPlist<SetupResponse>, StatusCode> {
    match req {
        freq @ SetupRequest::SenderInfo { .. } => {
            tracing::info!(?freq, "setup sender's info");
            Err(StatusCode::OK)
        }

        SetupRequest::Streams { streams } => {
            tracing::info!(?streams, "setup streams");
            Ok(BinaryPlist(SetupResponse::Streams { streams: vec![] }))
        }
    }
}

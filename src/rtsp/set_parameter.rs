use axum::{
    extract::{Path, State},
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use hyper::{header::CONTENT_TYPE, HeaderMap, StatusCode};
use tracing::{error, info};

use super::state::Connections;

pub async fn handler(
    Path(media_id): Path<String>,
    State(Connections(connections)): State<Connections>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // Volume, progress
    // Image
    // DMAP
    let connection = connections.entry(media_id).or_default().downgrade();
    match headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes) {
        Some(b"text/parameters") => {
            let body = String::from_utf8_lossy(&body);
            // TODO : set volume
            ().into_response()
        }
        Some(b"image/jpeg") => {
            info!(len = body.len(), "image received");
            ().into_response()
        }
        Some(b"application/x-dmap-tagged") => {
            info!(len = body.len(), "dmap received");
            ().into_response()
        }
        Some(other) => {
            let content_type = String::from_utf8_lossy(other);
            error!(%content_type, "unsupported content-type");
            (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported content-type",
            )
                .into_response()
        }
        None => (StatusCode::BAD_REQUEST, "empty content-type header").into_response(),
    }
}

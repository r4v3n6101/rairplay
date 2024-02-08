use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use hyper::{header::CONTENT_TYPE, StatusCode};
use tracing::error;

use super::state::Connections;

pub async fn handler(
    Path(media_id): Path<String>,
    State(Connections(connections)): State<Connections>,
    body: String,
) -> Response {
    match connections.get(&media_id) {
        Some(connection) => match body.as_str() {
            "volume\r\n" => (
                [(CONTENT_TYPE, "text/parameters")],
                format!("volume: {}\r\n", connection.volume.load()),
            )
                .into_response(),
            param => {
                error!(?param, "unimplemented parameter");
                (StatusCode::NOT_IMPLEMENTED, "unknown parameter").into_response()
            }
        },
        None => (StatusCode::NOT_FOUND, "connection not found").into_response(),
    }
}

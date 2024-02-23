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
) -> Result<Response, StatusCode> {
    let connection = connections.entry(media_id).or_default().downgrade();
    match body.as_str() {
        "volume\r\n" => Ok((
            [(CONTENT_TYPE, "text/parameters")],
            format!("volume: {}\r\n", connection.volume.load()),
        )
            .into_response()),
        param => {
            error!(?param, "unimplemented parameter");
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    }
}

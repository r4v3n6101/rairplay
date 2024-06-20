use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use hyper::{header::CONTENT_TYPE, StatusCode};

use crate::rtsp::state::SharedState;

// TODO
pub async fn handler(
    State(SharedState { state, .. }): State<SharedState>,
    body: String,
) -> Result<Response, StatusCode> {
    match body.as_str() {
        "volume\r\n" => Ok((
            [(CONTENT_TYPE, "text/parameters")],
            "volume: 0.0\r\n",
        )
            .into_response()),
        param => {
            tracing::error!(?param, "unimplemented parameter");
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    }
}

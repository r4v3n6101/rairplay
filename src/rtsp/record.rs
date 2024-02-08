use axum::response::IntoResponse;
use hyper::StatusCode;

// TODO : ???
pub async fn handler() -> impl IntoResponse {
    StatusCode::OK
}

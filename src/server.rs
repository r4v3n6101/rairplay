use axum::{extract::Request, response::IntoResponse, routing::options, Router};
use hyper::StatusCode;

pub struct AirplayServer;

impl AirplayServer {
    pub fn new() -> Router<()> {
        let router = Router::<()>::new()
            .route("/rtsp", options(options_handler))
            .fallback(invalid_request);

        router
    }
}

async fn options_handler(req: Request) -> impl IntoResponse {
    println!("Options called: {:?}", req);

    (StatusCode::NOT_FOUND, [("Upgrade", "RTSP")])
}

async fn invalid_request() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, [("Upgrade", "RTSP")])
}

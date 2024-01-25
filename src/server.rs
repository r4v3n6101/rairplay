use axum::{
    routing::{get, options},
    Router,
};

pub struct AirplayServer;

impl AirplayServer {
    pub fn new() -> Router<()> {
        let mut router = Router::<()>::new()
            .route("/info", get(info))
            .route("*", options(options_handler));

        router
    }
}

async fn options_handler() {
    println!("Options called");
}

async fn info() {
    println!("hi there");
}

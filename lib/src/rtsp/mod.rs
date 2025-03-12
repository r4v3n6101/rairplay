use std::{
    net::SocketAddr,
    task::{Context, Poll},
};

use axum::{
    extract::{connect_info::IntoMakeServiceWithConnectInfo, Request},
    handler::Handler,
    http::HeaderName,
    routing::{any, get, post},
    Router,
};
use state::SharedState;
use tower::Service;
use tower_http::propagate_header::PropagateHeaderLayer;

use crate::config::Config;

mod dto;
mod extractor;
mod handlers;
mod state;

pub struct RouterService {
    inner: IntoMakeServiceWithConnectInfo<Router<()>, SocketAddr>,
}

impl RouterService {
    pub fn serve(cfg: Config) -> Self {
        let state = SharedState::with_config(cfg);
        let inner = Router::new()
            // Heartbeat
            .route("/feedback", post(()))
            // I guess it will never be used
            .route("/command", post(()))
            // General info about server
            .route("/info", get(handlers::info))
            // Legacy pairing (shared secret via ecdh)
            .route("/pair-setup", post(handlers::pair_setup))
            .route("/pair-verify", post(handlers::pair_verify))
            // Fair play, but seems like it isn't working correct
            .route("/fp-setup", post(handlers::fp_setup))
            // Unknown handlers, just trace response
            .fallback(handlers::generic)
            // State cloned here, because it will be moved below
            .with_state(state.clone())
            // Custom RTSP methods
            .route(
                "/{media_id}",
                any(|req: Request| async move {
                    match req.method().as_str() {
                        // This is empty and useless
                        "RECORD" => handlers::generic.call(req, state).await,
                        "SETUP" => handlers::setup.call(req, state).await,
                        "GET_PARAMETER" => handlers::get_parameter.call(req, state).await,
                        "SET_PARAMETER" => handlers::set_parameter.call(req, state).await,
                        "FLUSH" => handlers::flush.call(req, state).await,
                        "FLUSHBUFFERED" => handlers::flush_buffered.call(req, state).await,
                        "SETRATEANCHORTIME" => {
                            handlers::set_rate_anchor_time.call(req, state).await
                        }
                        "TEARDOWN" => handlers::teardown.call(req, state).await,
                        method => {
                            tracing::warn!(?method, path = ?req.uri(), "unknown method");
                            handlers::generic.call(req, state).await
                        }
                    }
                }),
            )
            // CSeq is required for RTSP protocol
            .layer(PropagateHeaderLayer::new(HeaderName::from_static("cseq")))
            .into_make_service_with_connect_info::<SocketAddr>();

        Self { inner }
    }
}

impl Service<SocketAddr> for RouterService {
    type Response =
        <IntoMakeServiceWithConnectInfo<Router<()>, SocketAddr> as Service<SocketAddr>>::Response;
    type Error =
        <IntoMakeServiceWithConnectInfo<Router<()>, SocketAddr> as Service<SocketAddr>>::Error;
    type Future =
        <IntoMakeServiceWithConnectInfo<Router<()>, SocketAddr> as Service<SocketAddr>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: SocketAddr) -> Self::Future {
        self.inner.call(req)
    }
}

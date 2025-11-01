use std::{
    net::SocketAddr,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    Router,
    extract::{Request, connect_info::IntoMakeServiceWithConnectInfo},
    handler::Handler,
    http::HeaderName,
    routing::{any, get, post},
    serve::{IncomingStream, Listener},
};
use state::SharedState;
use tower::Service;
use tower_http::propagate_header::PropagateHeaderLayer;

use crate::{
    config::Config,
    playback::{audio::AudioDevice, video::VideoDevice},
};

mod dto;
mod extractor;
mod handlers;
mod state;

pub struct RtspService<A, V> {
    pub config: Arc<Config<A, V>>,
}

impl<L, A, V> Service<IncomingStream<'_, L>> for RtspService<A, V>
where
    L: Listener<Addr = SocketAddr>,
    A: AudioDevice,
    V: VideoDevice,
{
    type Response =
        <IntoMakeServiceWithConnectInfo<Router<()>, SocketAddr> as Service<SocketAddr>>::Response;
    type Error =
        <IntoMakeServiceWithConnectInfo<Router<()>, SocketAddr> as Service<SocketAddr>>::Error;
    type Future =
        <IntoMakeServiceWithConnectInfo<Router<()>, SocketAddr> as Service<SocketAddr>>::Future;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: IncomingStream<'_, L>) -> Self::Future {
        let state = SharedState::with_config(Arc::clone(&self.config));

        Router::new()
            // Heartbeat
            .route("/feedback", post(()))
            // I guess it will never be used
            .route("/command", post(()))
            // General info about server
            .route("/info", get(handlers::info))
            // Pairing
            .route("/pair-setup", post(handlers::pair_setup))
            .route("/pair-verify", post(handlers::pair_verify))
            // Fair play, for additional encryption of keys
            .route("/fp-setup", post(handlers::fp_setup))
            // Unknown handlers' response will be just traced
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
            .into_make_service_with_connect_info()
            .call(*req.remote_addr())
    }
}

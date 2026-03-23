use std::{
    convert::Infallible,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    Extension, Router,
    extract::{ConnectInfo, Request},
    handler::Handler,
    http::HeaderName,
    routing::{any, get, post},
    serve::IncomingStream,
};
use futures::{FutureExt, future::BoxFuture};
use tower::Service;
use tower_http::propagate_header::PropagateHeaderLayer;

use crate::{
    config::{Config, Keychain},
    playback::{audio::AudioDevice, video::VideoDevice},
    transport::DualStackListenerWithRtspRemap,
};

mod dto;
mod extractor;
mod handlers;
mod state;

/// Explicit type, so it could be stored somewhere
pub struct ServiceFactory<A, V, K> {
    pub config: Arc<Config<A, V, K>>,
}

impl<A, V, K> Service<IncomingStream<'_, DualStackListenerWithRtspRemap>>
    for ServiceFactory<A, V, K>
where
    A: AudioDevice,
    V: VideoDevice,
    K: Keychain,
{
    type Response = Router<()>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: IncomingStream<'_, DualStackListenerWithRtspRemap>) -> Self::Future {
        let config = Arc::clone(&self.config);
        let conn = req.remote_addr().clone();
        async move {
            let state = Arc::new(state::ServiceState::new(config));
            Ok(Router::new()
                // Heartbeat
                .route("/feedback", post(()))
                // I guess it will never be used
                .route("/command", post(()))
                // General info about server
                .route("/info", get(handlers::info))
                // Fair play, for additional encryption of keys
                .route("/fp-setup", post(handlers::fp_setup))
                // Unknown handlers' response will be just traced
                .fallback(handlers::generic)
                // State cloned here, because it will be moved below
                .with_state(Arc::clone(&state))
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
                .layer(Extension(ConnectInfo(conn))))
        }
        .boxed()
    }
}

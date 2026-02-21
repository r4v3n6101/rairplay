use std::{convert::Infallible, sync::Arc};

use axum::{
    Extension, Router,
    extract::{ConnectInfo, Request},
    handler::Handler,
    http::HeaderName,
    response::Response,
    routing::{any, get, post},
    serve::IncomingStream,
};
use tower::{Service, service_fn};
use tower_http::propagate_header::PropagateHeaderLayer;
pub use transport::TcpListenerWithRtspRemap as Listener;
use yoke::Yoke;

use crate::{
    config::{Config, Keychain, Pairing},
    pairing,
    playback::{audio::AudioDevice, video::VideoDevice},
};

mod dto;
mod extractor;
mod handlers;
mod state;
mod transport;

pub fn service_factory<A, V, K>(
    config: Arc<Config<A, V, K>>,
) -> impl for<'a> Service<
    IncomingStream<'a, Listener>,
    Response = impl Service<
        Request,
        Response = Response,
        Error = Infallible,
        Future = impl Future + Send + use<A, V, K>,
    > + Send
               + Clone
               + use<A, V, K>,
    Error = Infallible,
    Future = impl Future + Send + use<A, V, K>,
>
where
    A: AudioDevice,
    V: VideoDevice,
    K: Keychain,
{
    service_fn(move |incoming: IncomingStream<'_, Listener>| {
        let config = Arc::clone(&config);
        let conn = incoming.remote_addr().clone();
        async move {
            let state = Arc::new(state::ServiceState::new(config));
            let mut router = Router::new()
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
                .with_state(Arc::clone(&state));

            let keychain = Yoke::attach_to_cart(Arc::clone(&state), |state| &state.config.keychain)
                .erase_arc_cart();
            match state.config.pairing {
                Pairing::Legacy => {
                    // TODO : session_key will try to establish upgrade, so it won't work
                    router =
                        router.merge(pairing::legacy::router(keychain, conn.session_key.clone()));
                }
                Pairing::HomeKit => {
                    router = router.merge(pairing::homekit::router(
                        keychain,
                        conn.session_key.clone(),
                        state.config.pin,
                    ));
                }
            }

            // Custom RTSP methods
            router = router.route(
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
            );

            // CSeq is required for RTSP protocol
            router = router.layer(PropagateHeaderLayer::new(HeaderName::from_static("cseq")));
            router = router.layer(Extension(ConnectInfo(conn)));

            Ok(router)
        }
    })
}

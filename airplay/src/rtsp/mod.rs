use std::{convert::Infallible, sync::Arc};

use axum::{
    Router,
    extract::Request,
    handler::Handler,
    http::HeaderName,
    response::Response,
    routing::{any, get, post},
    serve::IncomingStream,
};
use tower::{Service, service_fn};
use tower_http::propagate_header::PropagateHeaderLayer;

use crate::{
    config::{Config, Pairing},
    pairing,
    playback::{audio::AudioDevice, video::VideoDevice},
};

pub use transport::TcpListenerWithRtspRemap as Listener;

mod dto;
mod extractor;
mod handlers;
mod state;
mod transport;

pub fn service_factory<A, V>(
    config: Arc<Config<A, V>>,
) -> impl for<'a> Service<
    IncomingStream<'a, Listener>,
    Response = impl Service<
        Request,
        Response = Response,
        Error = Infallible,
        Future = impl Future + Send + use<A, V>,
    > + Send
               + Clone
               + use<A, V>,
    Error = Infallible,
    Future = impl Future + Send + use<A, V>,
>
where
    A: AudioDevice,
    V: VideoDevice,
{
    service_fn(move |_: IncomingStream<'_, Listener>| {
        let config = Arc::clone(&config);
        async {
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

            // Pairing
            match state.config.pairing {
                Pairing::Legacy { pairing_key } => {
                    router = router.merge(pairing::legacy::router(
                        Arc::clone(&state) as Arc<dyn pairing::SessionKeyHolder>,
                        pairing_key,
                    ));
                }
                _ => {
                    // TODO
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

            Ok(router)
        }
    })
}

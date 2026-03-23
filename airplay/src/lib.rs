use std::{
    convert::Infallible,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{Router, serve::IncomingStream};
use futures::{FutureExt, future::BoxFuture};
use tower::Service;
use yoke::Yoke;

pub mod config;
pub mod playback;
pub mod transport;

pub(crate) mod crypto;
pub(crate) mod pairing;
pub(crate) mod rtsp;
pub(crate) mod streaming;

pub struct ServiceWithPairing<A, V, K> {
    inner: rtsp::ServiceFactory<A, V, K>,
}

impl<A, V, K> ServiceWithPairing<A, V, K>
where
    K: config::Keychain,
    A: playback::audio::AudioDevice,
    V: playback::video::VideoDevice,
{
    pub fn new(config: Arc<config::Config<A, V, K>>) -> Self {
        // TODO : verify features and append if neccessary
        Self {
            inner: rtsp::ServiceFactory { config },
        }
    }
}

impl<A, V, K> Service<IncomingStream<'_, transport::DualStackListenerWithRtspRemap>>
    for ServiceWithPairing<A, V, K>
where
    A: playback::audio::AudioDevice,
    V: playback::video::VideoDevice,
    K: config::Keychain,
{
    type Response = Router<()>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(
        &mut self,
        req: IncomingStream<'_, transport::DualStackListenerWithRtspRemap>,
    ) -> Self::Future {
        let pairing = self.inner.config.pairing;
        let pin = self.inner.config.pin;
        let keychain =
            Yoke::attach_to_cart(Arc::clone(&self.inner.config), |config| &config.keychain)
                .erase_arc_cart();
        let session_key = req.remote_addr().session_key.clone();

        let fut = self.inner.call(req);
        async move {
            let router = fut.await?;
            Ok(match pairing {
                config::Pairing::Legacy => {
                    router.merge(pairing::legacy::router(keychain, session_key))
                }
                config::Pairing::HomeKit => {
                    router.merge(pairing::homekit::router(keychain, session_key, pin))
                }
            })
        }
        .boxed()
    }
}

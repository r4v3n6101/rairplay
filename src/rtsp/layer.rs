use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, TryFutureExt};
use rtsp_types::{Request, Response};
use tower::{Layer, Service};

use crate::auth::rsa::auth_with_challenge;

#[derive(Debug, Clone)]
pub struct RsaAuth<I> {
    inner: I,
}

impl<I, B> Service<Request<B>> for RsaAuth<I>
where
    B: AsRef<[u8]>,
    I: Service<Request<B>, Response = Response<Vec<u8>>>,
    I::Future: 'static,
{
    type Response = I::Response;
    type Error = I::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let auth_token = req
            .header(&"Apple-Challenge".try_into().unwrap())
            .and_then(|challenge| {
                if let Ok(token) = auth_with_challenge(challenge.as_str()) {
                    // TODO : log?
                    Some(token)
                } else {
                    // TODO : trace
                    None
                }
            });

        let resp = self.inner.call(req);
        Box::pin(resp.and_then(|mut resp| async {
            if let Some(token) = auth_token {
                resp.append_header("Apple-Response".try_into().unwrap(), token);
            }

            Ok(resp)
        }))
    }
}

#[derive(Debug, Clone)]
pub struct RsaAuthLayer;

impl<S> Layer<S> for RsaAuthLayer
where
    S: Service<Request<Vec<u8>>>,
{
    type Service = RsaAuth<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RsaAuth { inner }
    }
}

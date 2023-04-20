use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
};

use futures::TryFutureExt;
use rtsp_types::{
    headers::{CSEQ, PUBLIC},
    Method, Request, Response, StatusCode, Version,
};
use tower::{Layer, Service};

use crate::auth::rsa::auth_with_challenge;

pub mod codec;

const SUPPORTED_OPTIONS: &str =
    "OPTIONS, ANNOUNCE, SETUP, RECORD, PAUSE, FLUSH, TEARDOWN, GET_PARAMETER, SET_PARAMETER";

pub type RtspRequest = Request<Vec<u8>>;
pub type RtspResponse = Response<Vec<u8>>;

#[derive(Debug, Clone)]
pub struct RtspService;

impl Service<RtspRequest> for RtspService {
    type Response = RtspResponse;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = io::Result<Self::Response>>>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Vec<u8>>) -> Self::Future {
        Box::pin(async move {
            let cseq = req.header(&CSEQ).cloned().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "CSEQ header not present")
            })?;
            let mut resp = match req.method() {
                Method::Options => Response::builder(Version::V1_0, StatusCode::Ok)
                    .header(PUBLIC, SUPPORTED_OPTIONS)
                    /*.header(
                        "Apple-Jack-Status".try_into().unwrap(),
                        "connected; type=analog",
                    )*/
                    .build(Vec::new()),
                Method::Announce => todo!("announce"),
                Method::Setup => todo!("setup"),
                _ => {
                    todo!("Invalid request: {:?}", req);
                }
            };

            // It must be present in the response and must equal to one in the request.
            resp.append_header(CSEQ, cseq);

            Ok(resp)
        })
    }
}

#[derive(Debug, Clone)]
pub struct RsaAuth<I> {
    inner: I,
}

impl<I> Service<RtspRequest> for RsaAuth<I>
where
    I: Service<RtspRequest, Response = RtspResponse, Error = io::Error>,
    I::Future: 'static,
{
    type Response = I::Response;
    type Error = I::Error;
    type Future = Pin<Box<dyn Future<Output = io::Result<Self::Response>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: RtspRequest) -> Self::Future {
        let auth_token = req
            .header(&"Apple-Challenge".try_into().unwrap())
            .map(|challenge| auth_with_challenge(challenge.as_str()));

        let resp = self.inner.call(req);
        Box::pin(async move {
            resp.and_then(|mut resp| async {
                if let Some(token) = auth_token {
                    resp.append_header("Apple-Response".try_into().unwrap(), token?);
                }

                Ok(resp)
            })
            .await
        })
    }
}

#[derive(Debug, Clone)]
pub struct RsaAuthLayer;

impl<S> Layer<S> for RsaAuthLayer
where
    S: Service<RtspRequest>,
{
    type Service = RsaAuth<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RsaAuth { inner }
    }
}

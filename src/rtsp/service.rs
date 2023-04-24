use std::{
    convert::Infallible,
    future::ready,
    pin::Pin,
    sync::Mutex,
    task::{Context, Poll},
};

use futures::Future;
use rtsp_types::{
    headers::{Public, Transports, CSEQ},
    Empty, Method, Request, Response, StatusCode, Version,
};
use sdp_types::Session;
use tower::Service;
use tracing::{debug, error, info, warn};

#[derive(Debug, Default)]
struct State {
    session: Option<Session>,
}

#[derive(Debug, Default)]
pub struct RtspService {
    state: Mutex<State>,
}

impl<I> Service<Request<I>> for RtspService
where
    I: AsRef<[u8]> + 'static,
{
    type Response = Response<Empty>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<I>) -> Self::Future {
        let Some(cseq) = req.header(&CSEQ).cloned() else {
            error!("CSEQ not present in headers");

            return Box::pin(ready(Ok(Response::builder(Version::V1_0, StatusCode::NotFound).empty())));
        };

        let mut resp = match req.method() {
            Method::Options => Response::builder(Version::V1_0, StatusCode::Ok)
                .typed_header(
                    &Public::builder()
                        .method(Method::Options)
                        .method(Method::Announce)
                        .method(Method::Setup)
                        .method(Method::Record)
                        .method(Method::Teardown)
                        .build(),
                )
                .empty(),
            Method::Announce => match Session::parse(req.body().as_ref()) {
                Ok(session) => {
                    info!(?session, "new session");
                    if let Some(old_session) = self.state.lock().unwrap().session.replace(session) {
                        warn!(?old_session, "session changed without TEARDOWN");
                    }

                    Response::builder(Version::V1_0, StatusCode::Ok).empty()
                }
                Err(err) => {
                    error!(%err, "SDP parsing failed");

                    Response::builder(Version::V1_0, StatusCode::BadRequest).empty()
                }
            },
            Method::Setup => {
                let transports = req.typed_header::<Transports>().unwrap(); // TODO : it's bug here
                println!("transport: {:?}", transports);

                Response::builder(Version::V1_0, StatusCode::Ok).empty()
            }
            Method::Teardown => {
                if let Some(old_session) = self.state.lock().unwrap().session.take() {
                    debug!(?old_session, "session closed");
                }

                Response::builder(Version::V1_0, StatusCode::Ok).empty()
            }
            _ => {
                warn!("unsupported method called");

                Response::builder(Version::V1_0, StatusCode::BadRequest).empty()
            }
        };

        // It must be present in the response and must equal to one in the request.
        resp.insert_header(CSEQ, cseq);
        // TODO : global context mb: resp.insert_header(SERVER, storage.name);

        Box::pin(ready(Ok(resp)))
    }
}

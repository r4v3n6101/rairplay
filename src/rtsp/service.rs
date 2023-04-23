use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, FutureExt};
use rtsp_types::{
    headers::{Public, Transports, CSEQ, SERVER},
    Empty, Method, Request, Response, StatusCode, Version,
};
use sdp_types::Session;
use tower::Service;

#[derive(Debug, Clone)]
pub struct RtspService {
    name: String,
}

impl Default for RtspService {
    fn default() -> Self {
        Self {
            name: "AirTunes/130.14".into(),
            //session: None,
        }
    }
}

impl RtspService {
    pub fn name(self, name: String) -> Self {
        Self { name, ..self }
    }
}

impl<I> Service<Request<I>> for RtspService
where
    I: AsRef<[u8]> + 'static,
{
    type Response = Response<Vec<u8>>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<I>) -> Self::Future {
        Box::pin(process_request(req, self.name.clone()).map(Ok))
    }
}

async fn process_request<B: AsRef<[u8]>>(
    req: Request<B>,
    service_name: String,
) -> Response<Vec<u8>> {
    let Some(cseq) = req.header(&CSEQ).cloned() else {
        // TODO : trace cseq not present
        return Response::builder(Version::V1_0, StatusCode::HeaderFieldNotValidForResource).build(Vec::new());
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
            .build(Vec::new()),
        Method::Announce => {
            match Session::parse(req.body().as_ref()) {
                Ok(session) => {
                    //if let Some(old_session) = self.session.replace(session) {
                    // TODO : just trace
                    //}
                    Response::builder(Version::V1_0, StatusCode::Ok).build(Vec::new())
                }
                Err(e) => {
                    // TODO : trace
                    Response::builder(Version::V1_0, StatusCode::BadRequest).build(Vec::new())
                }
            }
        }
        Method::Setup => {
            let transports = req.typed_header::<Transports>().unwrap(); // TODO : it's bug here
            println!("transport: {:?}", transports);

            todo!()
        }
        Method::Record => {
            unimplemented!("record not supported")
        }
        _ => {
            println!("Unsupported request: {:?}", req.replace_body(Empty));
            unimplemented!()
        }
    };

    // It must be present in the response and must equal to one in the request.
    resp.insert_header(CSEQ, cseq);
    resp.insert_header(SERVER, service_name);

    resp
}

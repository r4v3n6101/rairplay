use std::{
    collections::BTreeMap,
    convert::Infallible,
    net::IpAddr,
    sync::Arc,
    task::{Context, Poll},
};

use futures::future::{self, Ready};
use rtsp_types::{
    headers::{
        Public, RtpLowerTransport, RtpProfile, RtpTransport, RtpTransportParameters, Transport,
        TransportMode, Transports, CONTENT_TYPE, CSEQ,
    },
    Method, Request, Response, StatusCode, Version,
};
use tower::Service;
use tracing::{error, info, warn};

use crate::{rtp::spawn_listener, session::ClientSession};

#[derive(Debug)]
pub struct RtspService {
    local_addr: IpAddr,
    peer_addr: IpAddr,
    client_session: Option<Arc<ClientSession>>,
}

impl RtspService {
    pub fn new(local_addr: IpAddr, peer_addr: IpAddr) -> Self {
        Self {
            local_addr,
            peer_addr,
            client_session: None,
        }
    }
}

impl<I> Service<Request<I>> for RtspService
where
    I: AsRef<[u8]> + 'static,
{
    type Response = Response<Vec<u8>>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<I>) -> Self::Future {
        let Some(cseq) = req.header(&CSEQ).cloned() else {
            error!("CSEQ not present in headers");
            return future::ok(Response::builder(Version::V1_0, StatusCode::BadRequest).build(Vec::new()));
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
                        .method(Method::SetParameter)
                        .method(Method::GetParameter)
                        .build(),
                )
                .build(Vec::new()),
            Method::Announce => match sdp_types::Session::parse(req.body().as_ref()) {
                Ok(session) => {
                    let session = Arc::new(ClientSession::default());
                    info!(?session, "new session");
                    if let Some(old_session) = self.client_session.replace(session) {
                        warn!(?old_session, "ANNOUNCE session without TEARDOWN");
                    }
                    Response::builder(Version::V1_0, StatusCode::Ok).build(Vec::new())
                }
                Err(err) => {
                    error!(%err, "SDP parsing failed");
                    Response::builder(Version::V1_0, StatusCode::BadRequest).build(Vec::new())
                }
            },
            Method::Teardown => {
                if let Some(old_session) = self.client_session.take() {
                    info!(?old_session, "session closed");
                } else {
                    warn!("TEARDOWN without existing session");
                }

                Response::builder(Version::V1_0, StatusCode::Ok).build(Vec::new())
            }
            Method::Setup => {
                let Some(session) = &self.client_session else {
                    error!("session not set");
                    return future::ok(Response::builder(Version::V1_0, StatusCode::SessionNotFound)
                        .build(Vec::new()));
                };
                if let Ok(Some(transports)) = req.typed_header::<Transports>() {
                    if let Some(Transport::Rtp(rtp)) = transports.first() {
                        // TODO : use ports from incoming rtp transport
                        let rtp_transport = spawn_listener(
                            Arc::downgrade(session),
                            self.local_addr,
                            self.peer_addr,
                        )
                        .unwrap(); // TODO :
                        let mut ports = BTreeMap::new();
                        ports.insert(
                            "server_port".to_string(),
                            Some(rtp_transport.audio_port.to_string()),
                        );
                        ports.insert(
                            "control_port".to_string(),
                            Some(rtp_transport.control_port.to_string()),
                        );
                        ports.insert(
                            "timing_port".to_string(),
                            Some(rtp_transport.timing_port.to_string()),
                        );
                        let rtp_transport = Transport::Rtp(RtpTransport {
                            profile: RtpProfile::Avp,
                            lower_transport: Some(RtpLowerTransport::Udp),
                            params: RtpTransportParameters {
                                unicast: true,
                                mode: vec![TransportMode::Record],
                                others: ports,
                                ..Default::default()
                            },
                        });

                        Response::builder(Version::V1_0, StatusCode::Ok)
                            .typed_header(&Transports::from(vec![rtp_transport]))
                            .build(Vec::new())
                    } else {
                        error!(?transports, "no supported transport");
                        Response::builder(Version::V1_0, StatusCode::UnsupportedTransport)
                            .build(Vec::new())
                    }
                } else {
                    error!("no transport header");
                    Response::builder(Version::V1_0, StatusCode::BadRequest).build(Vec::new())
                }
            }
            Method::GetParameter => {
                // TODO : remove

                Response::builder(Version::V1_0, StatusCode::Ok)
                    .header(CONTENT_TYPE, "text/parameters")
                    .build(b"volume: 0.000000\r\n".to_vec())
            }
            Method::SetParameter | Method::Record | Method::Extension(_) => {
                // TODO : remove

                Response::builder(Version::V1_0, StatusCode::Ok).build(Vec::new())
            }
            _ => {
                warn!(method = ?req.method(), "unsupported method called");

                Response::builder(Version::V1_0, StatusCode::BadRequest).build(Vec::new())
            }
        };

        // It must be present in the response and must equal to the one in the request.
        resp.insert_header(CSEQ, cseq);

        future::ok(resp)
    }
}

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
use tokio::sync::RwLock;
use tower::Service;
use tracing::{error, info, warn};

use crate::{
    audio::{AudioCipher, AudioSink},
    rtp::spawn_listener,
};

#[derive(Debug)]
pub struct RtspService<A> {
    local_addr: IpAddr,
    peer_addr: IpAddr,
    audio_sink: Option<Arc<RwLock<A>>>,
    audio_cipher: Option<Box<dyn AudioCipher + Send>>,
}

impl<A> RtspService<A> {
    pub fn new(local_addr: IpAddr, peer_addr: IpAddr) -> Self {
        Self {
            local_addr,
            peer_addr,
            audio_sink: None,
            audio_cipher: None,
        }
    }
}

impl<I, A> Service<Request<I>> for RtspService<A>
where
    I: AsRef<[u8]> + 'static,
    A: AudioSink + Send + Sync + Unpin + 'static,
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
                    // TODO : replace from session info
                    let audio_sink = Arc::new(RwLock::new(A::initialize(0, 0, 0)));
                    // TODO : log params
                    info!("create new audio sink");
                    if self.audio_sink.replace(audio_sink).is_some() {
                        warn!("created new sink without tearing down previous");
                    }
                    Response::builder(Version::V1_0, StatusCode::Ok).build(Vec::new())
                }
                Err(err) => {
                    error!(%err, "SDP parsing failed");
                    Response::builder(Version::V1_0, StatusCode::BadRequest).build(Vec::new())
                }
            },
            Method::Teardown => {
                if self.audio_sink.take().is_some() {
                    info!("audio sink closed");
                } else {
                    warn!("no audio sink for tearing down");
                }

                Response::builder(Version::V1_0, StatusCode::Ok).build(Vec::new())
            }
            Method::Setup => {
                let Some(audio_sink) = &self.audio_sink else {
                    error!("audio sink isn't initialized");
                    return future::ok(Response::builder(Version::V1_0, StatusCode::SessionNotFound)
                        .build(Vec::new()));
                };
                if let Ok(Some(transports)) = req.typed_header::<Transports>() {
                    if let Some(Transport::Rtp(rtp)) = transports.first() {
                        // TODO : use ports from incoming rtp transport
                        let rtp_transport = spawn_listener(
                            self.local_addr,
                            self.peer_addr,
                            Arc::downgrade(audio_sink),
                            self.audio_cipher.take(),
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

use std::{
    collections::{BTreeMap, HashMap},
    convert::Infallible,
    net::IpAddr,
    str,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::future::{self, Ready};
use rtsp_types::{
    headers::{
        Public, RtpLowerTransport, RtpProfile, RtpTransport, RtpTransportParameters, Transport,
        TransportMode, Transports, CONTENT_TYPE, CSEQ,
    },
    Method, Request, Response, StatusCode, Version,
};
use tower::Service;
use tracing::{debug, error, info, warn};

use crate::crypto;

use super::session::{CodecFormat, Session};

macro_rules! scan {
    ( $string:expr, $sep:expr, $( $x:ty ),+ ) => {{
        let mut iter = $string.split($sep);
        ($(iter.next().and_then(|word| word.parse::<$x>().ok()),)*)
    }}
}

type VecResponse = Response<Vec<u8>>;

#[inline]
fn resp(code: StatusCode) -> VecResponse {
    Response::builder(Version::V1_0, code).build(Vec::new())
}

#[inline]
fn get_session_id<B>(req: &Request<B>) -> Result<&str, VecResponse> {
    req.request_uri().map(|u| &u.path()[1..]).ok_or_else(|| {
        error!("request uri is empty");
        resp(StatusCode::BadRequest)
    })
}

pub struct RtspService {
    addr: IpAddr,
    mac_addr: [u8; 6],
    sessions: HashMap<String, Arc<Session>>,
}

impl RtspService {
    pub fn new(addr: IpAddr, mac_addr: [u8; 6]) -> Self {
        Self {
            addr,
            mac_addr,
            sessions: Default::default(),
        }
    }

    fn handle_options(&self) -> VecResponse {
        Response::builder(Version::V1_0, StatusCode::Ok)
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
            .build(Vec::new())
    }

    fn handle_announce<B: AsRef<[u8]>>(&mut self, req: &Request<B>) -> VecResponse {
        #[inline]
        fn parse_alac_fmtp(input: &str) -> Option<CodecFormat> {
            let params = scan!(
                input,
                char::is_whitespace,
                u8,
                u32,
                u8,
                u8,
                u8,
                u8,
                u8,
                u8,
                u16,
                u32,
                u32,
                u32
            );
            Some(CodecFormat::ALAC {
                frame_len: params.1?,
                compatible_version: params.2?,
                bit_depth: params.3?,
                pb: params.4?,
                mb: params.5?,
                kb: params.6?,
                channels_num: params.7?,
                max_run: params.8?,
                max_frame_bytes: params.9?,
                avg_bit_rate: params.10?,
                sample_rate: params.11?,
            })
        }

        match sdp_types::Session::parse(req.body().as_ref()) {
            Ok(session) => {
                let codec_fmt = match session.get_first_attribute_value("fmtp") {
                    // TODO : it may be not ALAC
                    Ok(Some(res)) => match parse_alac_fmtp(res) {
                        Some(x) => x,
                        None => {
                            error!("some of alac parameters're missing");
                            return resp(StatusCode::BadRequest);
                        }
                    },
                    Ok(None) => {
                        error!("empty fmtp attribute");
                        return resp(StatusCode::BadRequest);
                    }
                    Err(_) => {
                        error!("fmtp attribute not provided");
                        return resp(StatusCode::BadRequest);
                    }
                };

                // TODO : it may be not rsaaeskey
                let (aes_key, aes_iv) = match (
                    session.get_first_attribute_value("fpaeskey"),
                    session.get_first_attribute_value("aesiv"),
                ) {
                    (Ok(Some(fpaeskey)), Ok(Some(aesiv))) => {
                        match crypto::rsaaeskey(fpaeskey, aesiv) {
                            Ok(x) => x,
                            Err(e) => {
                                error!(%e, "error parsing either aes or iv");
                                return resp(StatusCode::BadRequest);
                            }
                        }
                    }
                    _ => {
                        error!("either aes or iv not provided");
                        return resp(StatusCode::BadRequest);
                    }
                };

                self.sessions.insert(
                    session.origin.sess_id,
                    Arc::new(Session::init(codec_fmt, aes_key, aes_iv)),
                );

                resp(StatusCode::Ok)
            }
            Err(err) => {
                error!(%err, "sdp parsing failed");
                resp(StatusCode::BadRequest)
            }
        }
    }

    fn handle_setup<B>(&self, req: &Request<B>) -> VecResponse {
        let Some(audio_sink) = &self.audio_sink else {
            error!("audio sink isn't initialized");
            return resp(StatusCode::SessionNotFound);
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
                resp(StatusCode::UnsupportedTransport)
            }
        } else {
            error!("no transport header");
            resp(StatusCode::BadRequest)
        }
    }

    fn handle_teardown<B>(&self, req: &Request<B>) -> VecResponse {
        let id = match get_session_id(req) {
            Ok(sess_id) => sess_id,
            Err(resp) => return resp,
        };
        if let Some(old_session) = self.sessions.remove(id) {
            info!(%id, ?old_session, "session closed");
        } else {
            warn!(%id, "no session to close");
        }
        resp(StatusCode::Ok)
    }

    fn handle_get_parameter<B>(&self, req: &Request<B>) -> VecResponse {
        let id = match get_session_id(req) {
            Ok(sess_id) => sess_id,
            Err(resp) => return resp,
        };
        todo!()
    }

    fn handle_set_parameter<B: AsRef<[u8]>>(&self, req: &Request<B>) -> VecResponse {
        let id = match get_session_id(req) {
            Ok(sess_id) => sess_id,
            Err(resp) => return resp,
        };
        let Some(session) = self.sessions.get(id) else {
            error!(%id, "session not found");
            return resp(StatusCode::SessionNotFound);
        };
        let Some(content_type) = req.header(&CONTENT_TYPE) else {
            error!("content type header not provided");
            return resp(StatusCode::BadRequest);
        };

        match content_type.as_str() {
            "text/parameters" => {
                let body = match str::from_utf8(req.body().as_ref()) {
                    Ok(s) => s,
                    Err(err) => {
                        error!(%err, "body must be utf-8");
                        return resp(StatusCode::BadRequest);
                    }
                };

                // TODO : read volume/progress and set
            }
            "image/jpeg" => {
                // TODO : get owned
                let img = Bytes::from(req.body().as_ref());
                debug!(len = img.len(), "new artwork");
                session.get_metadata().set_artwork(img);
            }
            "application/x-dmap-tagged" => {
                // TODO
            }
            other => {
                warn!(content_type = other, "unknown content type");
            }
        }

        resp(StatusCode::Ok)
    }
}

impl<I> Service<Request<I>> for RtspService
where
    I: AsRef<[u8]> + 'static,
{
    type Response = VecResponse;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<I>) -> Self::Future {
        let Some(cseq) = req.header(&CSEQ).cloned() else {
            error!("CSEQ not present in headers");
            return future::ok(resp(StatusCode::BadRequest));
        };

        let auth_token = match req.header(&"Apple-Challenge".try_into().unwrap()) {
            Some(challenge) => {
                match crypto::auth_with_challenge(challenge.as_str(), &self.addr, &self.mac_addr) {
                    Ok(token) => {
                        info!(token, "authenticated connection");
                        Some(token)
                    }
                    Err(e) => {
                        error!(%e, "couldn't authenticate");
                        return future::ok(resp(StatusCode::Unauthorized));
                    }
                }
            }
            None => None,
        };

        let mut resp = match req.method() {
            Method::Options => self.handle_options(),
            Method::Announce => self.handle_announce(&req),
            Method::Teardown => self.handle_teardown(&req),
            Method::Setup => self.handle_setup(&req),
            Method::GetParameter => self.handle_get_parameter(&req),
            Method::SetParameter => self.handle_set_parameter(&req),
            Method::Record | Method::Extension(_) => {
                // TODO : remove

                resp(StatusCode::Ok)
            }
            _ => {
                warn!(method = ?req.method(), "unsupported method called");

                resp(StatusCode::BadGateway)
            }
        };

        // It must be present in the response and must equal to the one in the request.
        resp.insert_header(CSEQ, cseq);
        if let Some(token) = auth_token {
            resp.append_header("Apple-Response".try_into().unwrap(), token);
        }

        future::ok(resp)
    }
}

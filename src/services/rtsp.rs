use std::{
    convert::Infallible,
    net::IpAddr,
    str,
    task::{Context, Poll},
};

use bytes::{BufMut, Bytes};
use futures::future::{self, Ready};
use mac_address2::MacAddress;
use rtsp_types::{
    headers::{Public, CONTENT_TYPE, CSEQ},
    HeaderValue, Method, Response, StatusCode, Version,
};
use symphonia_core::codecs::{CodecParameters, CODEC_TYPE_ALAC};
use tower::Service;
use tracing::{debug, error, info, warn};

use crate::{
    codecs::rtsp::{RtspRequest, RtspResponse},
    crypto::{self, KeyEncryption},
    state::{State, StateStorage},
};

// TODO : separate crate
macro_rules! scan {
    ( $string:expr, $sep:expr, $( $x:ty ) + ) => {{
        let mut iter = $string.split($sep);
        ($(iter.next().and_then(|word| word.parse::<$x>().ok()),)*)
    }}
}

#[inline]
fn resp(code: StatusCode) -> RtspResponse {
    Response::builder(Version::V1_0, code).build(Vec::new())
}

#[inline]
fn get_session_id(req: &RtspRequest) -> Result<&str, RtspResponse> {
    req.request_uri().map(|u| &u.path()[1..]).ok_or_else(|| {
        error!("request uri is empty");
        resp(StatusCode::BadRequest)
    })
}

#[inline]
fn parse_alac_fmtp(input: &str) -> Option<CodecParameters> {
    let params = scan!(
        input,
        char::is_whitespace,
        u8 u32 u8 u8 u8 u8 u8 u8 u16 u32 u32 u32
    );

    let mut magic_cookie = Box::new([0u8; 24]);
    {
        let mut buf = magic_cookie.as_mut_slice();
        buf.put_u32(params.1?);
        buf.put_u8(params.2?);
        buf.put_u8(params.3?);
        buf.put_u8(params.4?);
        buf.put_u8(params.5?);
        buf.put_u8(params.6?);
        buf.put_u8(params.7?);
        buf.put_u16(params.8?);
        buf.put_u32(params.9?);
        buf.put_u32(params.10?);
        buf.put_u32(params.11?);
    }

    // TODO : fill extra fields?
    Some(CodecParameters {
        codec: CODEC_TYPE_ALAC,
        extra_data: Some(magic_cookie),
        ..Default::default()
    })
}

pub struct RtspService {
    addr: IpAddr,
    mac_addr: MacAddress,
    storage: StateStorage,
}

impl RtspService {
    pub fn new(addr: IpAddr, mac_addr: MacAddress) -> Self {
        Self {
            addr,
            mac_addr,
            storage: Default::default(),
        }
    }

    fn handle_options(&self) -> RtspResponse {
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

    fn handle_announce(&mut self, req: &RtspRequest) -> RtspResponse {
        match sdp_types::Session::parse(req.body().as_ref()) {
            Ok(session) => {
                let codec_params = match session.get_first_attribute_value("fmtp") {
                    // TODO : it may be not ALAC
                    Ok(Some(res)) => match parse_alac_fmtp(res) {
                        Some(x) => x,
                        None => {
                            error!("some of alac parameters're missing");
                            return resp(StatusCode::BadRequest);
                        }
                    },
                    _ => {
                        error!("fmtp attribute not provided");
                        return resp(StatusCode::BadRequest);
                    }
                };

                let encryption = match (
                    session.get_first_attribute_value("fpaeskey"),
                    session.get_first_attribute_value("aesiv"),
                ) {
                    (Ok(Some(fpaeskey)), Ok(Some(aesiv))) => {
                        // TODO : it may be not rsaaeskey
                        KeyEncryption::Rsa {
                            aeskey64: fpaeskey.to_string(),
                            aesiv64: aesiv.to_string(),
                        }
                    }
                    _ => {
                        error!("either aes or iv not provided");
                        return resp(StatusCode::BadRequest);
                    }
                };

                self.storage.insert(
                    session.origin.sess_id,
                    State::Announced {
                        codec_params,
                        encryption,
                    },
                );

                resp(StatusCode::Ok)
            }
            Err(err) => {
                error!(%err, "sdp parsing failed");
                resp(StatusCode::BadRequest)
            }
        }
    }

    fn handle_setup(&self, req: &RtspRequest) -> RtspResponse {
        todo!("implement")
    }

    fn handle_teardown(&self, req: &RtspRequest) -> RtspResponse {
        let id = match get_session_id(req) {
            Ok(sess_id) => sess_id,
            Err(resp) => return resp,
        };
        self.storage.remove(id);

        resp(StatusCode::Ok)
    }

    fn handle_get_parameter(&self, req: &RtspRequest) -> RtspResponse {
        let id = match get_session_id(req) {
            Ok(sess_id) => sess_id,
            Err(resp) => return resp,
        };
        let body = match str::from_utf8(req.body()) {
            Ok(s) => s,
            Err(err) => {
                error!(%err, "body must be utf-8");
                return resp(StatusCode::BadRequest);
            }
        };
        debug!(%body, "requested parameter");

        if body.contains("volume") {
            if let Some(volume) = self.storage.fetch_metadata(id, |meta| meta.volume) {
                Response::builder(Version::V1_0, StatusCode::Ok)
                    .header(CONTENT_TYPE, "text/parameters")
                    .build(format!("volume: {}", volume).into_bytes())
            } else {
                error!(%id, "non-initialized stream");
                resp(StatusCode::NotFound)
            }
        } else if body.contains("progress") {
            if let Some((begin, current, end)) = self
                .storage
                .fetch_metadata(id, |meta| (meta.begin_pos, meta.current_pos, meta.end_pos))
            {
                Response::builder(Version::V1_0, StatusCode::Ok)
                    .header(CONTENT_TYPE, "text/parameters")
                    .build(format!("progress: {}/{}/{}", begin, current, end).into_bytes())
            } else {
                error!(%id, "non-initialized stream");
                resp(StatusCode::NotFound)
            }
        } else {
            warn!("unknown parameter");
            resp(StatusCode::BadRequest)
        }
    }

    fn handle_set_parameter(&self, req: RtspRequest) -> RtspResponse {
        let id = match get_session_id(&req).map(ToOwned::to_owned) {
            Ok(sess_id) => sess_id,
            Err(resp) => return resp,
        };
        let Some(content_type) = req
            .header(&CONTENT_TYPE)
            .map(HeaderValue::as_str)
            .map(ToOwned::to_owned)
        else {
            error!("Content-Type not provided");
            return resp(StatusCode::BadRequest);
        };
        let body = req.into_body();

        match content_type.as_str() {
            "text/parameters" => {
                let body = match str::from_utf8(&body) {
                    Ok(s) => s,
                    Err(err) => {
                        error!(%err, "body must be utf-8");
                        return resp(StatusCode::BadRequest);
                    }
                };
                debug!(%body, "text body");

                if let Some(volume) = body.strip_prefix("volume: ") {
                    let Ok(volume) = volume.parse::<f64>() else {
                        error!("invalid volume float");
                        return resp(StatusCode::BadRequest);
                    };

                    self.storage
                        .update_metadata(&id, |meta| meta.volume = volume);
                } else if let Some(progress) = body.strip_prefix("progress: ") {
                    let (Some(begin), Some(current), Some(end)) =
                        scan!(progress, |c| c == '/', u32 u32 u32)
                    else {
                        error!("invalid progress format");
                        return resp(StatusCode::BadRequest);
                    };

                    self.storage.update_metadata(&id, |meta| {
                        meta.begin_pos = begin;
                        meta.current_pos = current;
                        meta.end_pos = end;
                    });
                } else {
                    warn!("unknown text format");
                }
            }
            "image/jpeg" => {
                let img = Bytes::from(body);
                debug!(len = img.len(), "new artwork");

                self.storage.update_metadata(&id, |meta| meta.artwork = img);
            }
            "application/x-dmap-tagged" => {
                // TODO : dmap parser for more metainfo
            }
            other => {
                warn!(content_type = other, "unknown content type");
            }
        }

        resp(StatusCode::Ok)
    }
}

impl Service<RtspRequest> for RtspService {
    type Response = RtspResponse;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RtspRequest) -> Self::Future {
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
            Method::SetParameter => self.handle_set_parameter(req),
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

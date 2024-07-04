use std::ops::Deref;

use async_trait::async_trait;
use axum::{
    extract::{rejection::BytesRejection, FromRequest, Request},
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use bytes::{BufMut, Bytes, BytesMut};
use hyper::{header::CONTENT_TYPE, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

const APPLE_BPLIST_MIME: &str = "application/x-apple-binary-plist";

#[derive(Debug, Error)]
pub enum PlistRejection {
    #[error(transparent)]
    Plist(#[from] plist::Error),
    #[error(transparent)]
    Bytes(#[from] BytesRejection),
}

impl IntoResponse for PlistRejection {
    fn into_response(self) -> Response {
        tracing::error!(err = %self, "plist error");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BinaryPlist<T>(pub T);

impl<T> BinaryPlist<T>
where
    T: DeserializeOwned,
{
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PlistRejection> {
        if tracing::event_enabled!(tracing::Level::TRACE) {
            match plist::from_bytes::<plist::Value>(bytes) {
                Ok(plist_value) => {
                    tracing::trace!(?plist_value, "deserialized plist value");
                }
                Err(err) => {
                    tracing::trace!(%err, "failed to represent deserialized plist value");
                }
            }
        }

        plist::from_bytes(bytes).map(Self).map_err(Into::into)
    }
}

impl<T> Deref for BinaryPlist<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for BinaryPlist<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

#[async_trait]
impl<T, S> FromRequest<S> for BinaryPlist<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = PlistRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state).await?;
        Self::from_bytes(&bytes)
    }
}

impl<T> IntoResponse for BinaryPlist<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        if tracing::event_enabled!(tracing::Level::TRACE) {
            match plist::to_value(&self.0) {
                Ok(plist_value) => {
                    tracing::trace!(?plist_value, "serialized plist value");
                }
                Err(err) => {
                    tracing::trace!(%err, "failed to represent serialized plist value");
                }
            }
        }

        let mut buf = BytesMut::with_capacity(1024).writer();
        match plist::to_writer_binary(&mut buf, &self.0) {
            Ok(()) => (
                [(CONTENT_TYPE, HeaderValue::from_static(APPLE_BPLIST_MIME))],
                buf.into_inner().freeze(),
            )
                .into_response(),
            Err(err) => PlistRejection::from(err).into_response(),
        }
    }
}

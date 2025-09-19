// mod handshake;

use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use http::StatusCode;

use super::{
    dto::{Method, PairSetup, PairSetupAuth},
    extractor::TaggedValue,
};

pub async fn pair_setup(bytes: Bytes) -> Response {
    // TODO : unsupported, probably never-ever will
    if let Ok(TaggedValue(())) = TaggedValue::<Method<PairSetupAuth>>::from_bytes(&bytes) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "MFi authentication not supported",
        )
            .into_response();
    }

    if let Err(err) = TaggedValue::<Method<PairSetup>>::from_bytes(&bytes) {
        return err.into_response();
    }

    todo!()
}

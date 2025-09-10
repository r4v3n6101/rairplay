use axum::{
    extract::{FromRequest, Request, rejection::BytesRejection},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::{HeaderValue, StatusCode, header::CONTENT_TYPE};
use thiserror::Error;

use super::dto::{TagCode, Tlv8Pack};

mod endec;

const APPLE_TLV8_MIME: &str = "application/pairing+tlv8";

#[derive(Debug, Error)]
pub enum Tlv8Rejection {
    #[error(transparent)]
    Bytes(#[from] BytesRejection),
    #[error(transparent)]
    Decoding(#[from] endec::DecodingError),
    #[error("missing tag: {0}")]
    MissingTag(TagCode),
}

impl IntoResponse for Tlv8Rejection {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TaggedValue<T: Tlv8Pack>(pub T::Value);

pub trait Tlv8Encode: Tlv8Pack {
    fn bytes_iter(value: Self::Value) -> impl Iterator<Item = u8>;
}

pub trait Tlv8Decode: Tlv8Pack {
    fn from_iter<'a, I>(iter: I) -> Result<Self::Value, Tlv8Rejection>
    where
        I: IntoIterator<Item = (TagCode, &'a [u8])> + Clone;
}

impl<T: Tlv8Encode> IntoResponse for TaggedValue<T> {
    fn into_response(self) -> Response {
        (
            [(CONTENT_TYPE, HeaderValue::from_static(APPLE_TLV8_MIME))],
            self.bytes().collect::<Bytes>(),
        )
            .into_response()
    }
}

impl<S: Send + Sync, T: Tlv8Decode> FromRequest<S> for TaggedValue<T> {
    type Rejection = Tlv8Rejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        Self::from_bytes(&Bytes::from_request(req, state).await?)
    }
}

impl<T: Tlv8Encode> TaggedValue<T> {
    pub fn bytes(self) -> impl Iterator<Item = u8> {
        T::bytes_iter(self.0)
    }
}

impl<T: Tlv8Decode> TaggedValue<T> {
    pub fn from_bytes(buf: &[u8]) -> Result<Self, Tlv8Rejection> {
        #[derive(Clone)]
        struct Iter<'a> {
            buf: &'a [u8],
        }

        impl<'a> Iterator for Iter<'a> {
            type Item = (TagCode, &'a [u8]);

            fn next(&mut self) -> Option<Self::Item> {
                while let Some((&[tag, len], remain)) = self.buf.split_first_chunk() {
                    // Out of bounds, so ends iterating
                    let Some((value, remain)) = remain.split_at_checked(len.into()) else {
                        break;
                    };
                    self.buf = remain;

                    // skip unknown tags
                    if let Some(tag) = TagCode::from_repr(tag) {
                        return Some((tag, value));
                    }
                }

                None
            }
        }

        T::from_iter(Iter { buf }).map(Self)
    }
}

#[cfg(test)]
mod tests {

    use super::{super::dto::*, *};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use bytes::Bytes;
    use futures::TryStreamExt;

    fn encode_tlv(tag: u8, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for chunk in data.chunks(0xFF) {
            out.push(tag);
            out.push(chunk.len() as u8);
            out.extend_from_slice(chunk);
        }
        out
    }

    #[tokio::test]
    async fn parse_public_key_from_request() {
        let data = vec![1, 2, 3, 4, 5];
        let encoded = encode_tlv(TagCode::PublicKey as _, &data);

        let request = Request::builder()
            .header(CONTENT_TYPE, APPLE_TLV8_MIME)
            .body(Body::from(encoded))
            .unwrap();

        let parsed: TaggedValue<PublicKey> = TaggedValue::from_request(request, &())
            .await
            .expect("Failed to parse");
        assert_eq!(parsed.0, data);
    }

    #[tokio::test]
    async fn round_trip_public_key() {
        let data = vec![10, 20, 30, 40, 50];
        let tagged = TaggedValue::<PublicKey>(data.clone());

        let response = tagged.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes: Vec<Bytes> = response
            .into_body()
            .into_data_stream()
            .try_collect()
            .await
            .expect("Failed to read body");
        let body_bytes: Vec<u8> = body_bytes.into_iter().flatten().collect();
        assert_eq!(body_bytes, encode_tlv(TagCode::PublicKey as _, &data));

        let request = Request::builder()
            .header(CONTENT_TYPE, APPLE_TLV8_MIME)
            .body(Body::from(body_bytes.clone()))
            .unwrap();

        let parsed: TaggedValue<PublicKey> = TaggedValue::from_request(request, &())
            .await
            .expect("Failed to parse");
        assert_eq!(parsed.0, data);
    }

    #[tokio::test]
    async fn parse_large_public_key_wrapped_chunks() {
        let data: Vec<u8> = (0..300).map(|i| (i % 256) as u8).collect();
        let encoded = encode_tlv(TagCode::PublicKey as _, &data);

        assert_eq!(encoded[0], TagCode::PublicKey as _, "First TLV tag");
        assert_eq!(encoded[1], 255, "First TLV length");
        assert_eq!(&encoded[2..2 + 255], &data[..255], "First TLV payload");

        let second_start = 2 + 255;
        assert_eq!(
            encoded[second_start],
            TagCode::PublicKey as _,
            "Second TLV tag"
        );
        assert_eq!(encoded[second_start + 1], 45, "Second TLV length");
        assert_eq!(
            &encoded[second_start + 2..second_start + 2 + 45],
            &data[255..],
            "Second TLV payload"
        );

        let request = Request::builder()
            .header(CONTENT_TYPE, APPLE_TLV8_MIME)
            .body(Body::from(encoded.clone()))
            .unwrap();

        let parsed: TaggedValue<PublicKey> = TaggedValue::from_request(request, &())
            .await
            .expect("Failed to parse wrapped TLV8");

        assert_eq!(parsed.0, data, "Parsed data matches original");

        let reencoded: Vec<u8> = PublicKey::bytes_iter(parsed.0.clone()).collect();
        assert_eq!(reencoded, encoded, "Re-encoded TLV matches original");
    }

    #[tokio::test]
    async fn parse_tuple_public_key_and_another_tag() {
        let pk_data = vec![1, 2, 3];
        let m_data = vec![method::PairSetup::CODE as _];
        let mut encoded = encode_tlv(TagCode::PublicKey as _, &pk_data);
        encoded.extend(encode_tlv(TagCode::Method as _, &m_data));

        let request = Request::builder()
            .header(CONTENT_TYPE, APPLE_TLV8_MIME)
            .body(Body::from(encoded))
            .unwrap();

        let _: TaggedValue<(PublicKey, Method<method::PairSetup>)> =
            TaggedValue::from_request(request, &())
                .await
                .expect("Failed to parse tuple");
    }
}

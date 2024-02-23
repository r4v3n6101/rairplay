use bytes::Bytes;
use tracing::trace;

pub async fn trace_body(bytes: Option<Bytes>) {
    trace!(?bytes, "body bytes");
}

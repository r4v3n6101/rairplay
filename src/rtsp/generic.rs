use bytes::Bytes;

pub async fn trace_body(bytes: Option<Bytes>) {
    tracing::trace!(?bytes);
}

use serde::Deserialize;

use crate::service::plist::BinaryPlist;

#[derive(Debug, Deserialize)]
pub struct FlushBufferedRequest {
    #[serde(rename = "flushUntilSeq")]
    flush_until_seqnum: Option<u32>,
    #[serde(rename = "flushUntilTS")]
    flush_until_timestamp: Option<u32>,
    #[serde(rename = "flushFromSeq")]
    flush_from_seqnum: Option<u32>,
    #[serde(rename = "flushFromTS")]
    flush_from_timestamp: Option<u32>,
}

pub async fn handler(obj: BinaryPlist<FlushBufferedRequest>) {
    tracing::debug!(?obj, "FLUSHBUFFERED");
}

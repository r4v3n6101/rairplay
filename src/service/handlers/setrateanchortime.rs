use serde::Deserialize;

use crate::service::plist::BinaryPlist;

#[derive(Debug, Deserialize)]
pub struct SetRateAnchorTimeRequest {
    rate: f32,
    #[serde(rename = "rtpTime")]
    anchor_rtp_timestamp: Option<u64>,
}

pub async fn handler(request: BinaryPlist<SetRateAnchorTimeRequest>) {
    // TODO
    tracing::debug!(?request);
}

use serde::Deserialize;

use crate::plist::BinaryPlist;

#[derive(Debug, Deserialize)]
pub struct SetRateAnchorTimeRequest {
    pub rate: f32,
    #[serde(rename = "rtpTime")]
    pub rtp_time: Option<u64>,
}

pub async fn handler(request: BinaryPlist<SetRateAnchorTimeRequest>) {
    // TODO
    tracing::debug!(?request);
}

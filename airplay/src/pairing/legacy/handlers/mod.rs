use std::sync::Arc;

use axum::{Extension, extract::State, response::IntoResponse};
use bytes::Bytes;
use http::StatusCode;
use inner::{SIGNATURE_LENGTH, X25519_KEY_LEN};

use super::{
    super::{SessionKey, SharedSessionKey},
    state::ServiceState,
};

pub mod inner;

/// Don't really need request body here, because it duplicates signing key of counterparty got in
/// the second request.
#[tracing::instrument(level = "DEBUG", ret, skip(state))]
pub async fn pair_setup(State(state): State<Arc<ServiceState>>) -> impl IntoResponse {
    state.pairing.lock().unwrap().verifying_key()
}

#[tracing::instrument(level = "DEBUG", ret, err, skip(state, session_key))]
pub async fn pair_verify(
    State(state): State<Arc<ServiceState>>,
    Extension(session_key): Extension<SharedSessionKey>,
    body: Bytes,
) -> Result<impl IntoResponse, StatusCode> {
    if body.len() < 4 + 2 * X25519_KEY_LEN {
        tracing::error!(len=%body.len(), "malformed data for legacy pairing");
        return Err(StatusCode::BAD_REQUEST);
    }

    let mode = body[0];
    let mut pairing_state = state.pairing.lock().unwrap();
    if mode > 0 {
        let pubkey_their = &body[4..][..X25519_KEY_LEN];
        let verify_their = &body[36..][..X25519_KEY_LEN];

        pairing_state
            .establish_agreement(rand::rng(), pubkey_their, verify_their)
            .inspect(|&(_, shared_secret)| {
                tracing::info!("agreement established");
                session_key.lock_write().replace(SessionKey {
                    key_material: shared_secret,
                    upgrade_channel: false,
                });
            })
            .inspect_err(|err| tracing::error!(%err, "establishing agreement failed"))
            .map(|(response, _)| response.into_response())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    } else {
        let signature = body[4..][..SIGNATURE_LENGTH].try_into().unwrap();
        pairing_state
            .verify_agreement(signature)
            .inspect(|()| tracing::info!("agreement verified"))
            .inspect_err(|err| tracing::warn!(%err, "agreement verification failed"))
            .map(|()| ().into_response())
            .map_err(|_| StatusCode::OK)
    }
}

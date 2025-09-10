use std::sync::Arc;

use axum::{Extension, extract::State, response::IntoResponse};
use bytes::Bytes;
use http::StatusCode;

use inner::{SIGNATURE_LENGTH, X25519_KEY_LEN};

use super::{super::SessionKeyHolder, state::ServiceState};

pub mod inner;

/// Don't really need request body here, because it duplicates signing key of counterparty got in
/// the second request.
pub async fn pair_setup(State(state): State<Arc<ServiceState>>) -> impl IntoResponse {
    state.pairing.lock().unwrap().verifying_key()
}

pub async fn pair_verify(
    State(state): State<Arc<ServiceState>>,
    Extension(key_holder): Extension<Arc<dyn SessionKeyHolder>>,
    body: Bytes,
) -> impl IntoResponse {
    if body.len() < 4 + 2 * X25519_KEY_LEN {
        tracing::error!(len=%body.len(), "malformed data for legacy pairing");
        return Err(StatusCode::BAD_REQUEST);
    }

    let mode = body[0];
    let mut pairing_state = state.pairing.lock().unwrap();
    let response = if mode > 0 {
        let pubkey_their = body[4..][..X25519_KEY_LEN].try_into().unwrap();
        let verify_their = body[36..][..X25519_KEY_LEN].try_into().unwrap();

        pairing_state
            .establish_agreement(pubkey_their, verify_their, rand::rng())
            .inspect_err(|err| tracing::error!(%err, "legacy pairing establishment failed"))
            .map(IntoResponse::into_response)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    } else {
        let signature = body[4..][..SIGNATURE_LENGTH].try_into().unwrap();

        pairing_state
            .verify_agreement(signature)
            .inspect_err(|err| tracing::error!(%err, "legacy pairing verification failed"))
            .map(IntoResponse::into_response)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    };

    if let Some(session_key) = pairing_state.shared_secret() {
        key_holder.set_session_key(session_key.to_vec().into());
    }

    response
}

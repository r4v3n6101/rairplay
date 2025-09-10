use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::StatusCode;

use super::{
    dto::{ErrorCode, Method, PairingFlags, PairingState, Proof, PublicKey, Salt, method, state},
    extractor::TaggedValue,
    state::ServiceState,
};

pub mod setup;
pub mod verify;

// PairingFlags are optional
type M1Msg = TaggedValue<(PairingState<state::M1>, Method<method::PairSetup>)>;
type M2Msg = TaggedValue<(PairingState<state::M2>, PublicKey, Salt, PairingFlags)>;
type M3Msg = TaggedValue<(PairingState<state::M3>, PublicKey, Proof)>;
type M4Msg = TaggedValue<(PairingState<state::M4>, Proof)>;
type ErrorResponse<S> = TaggedValue<(PairingState<S>, ErrorCode)>;

pub async fn pair_setup(
    State(state): State<Arc<ServiceState>>,
    bytes: Bytes,
) -> Result<Response, Response> {
    // NB : unsupported, probably never-ever will
    let Err(_) = TaggedValue::<Method<method::PairSetupAuth>>::from_bytes(&bytes) else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "MFi authentication not supported",
        )
            .into_response());
    };

    if M1Msg::from_bytes(&bytes).is_ok() {
        let flags = TaggedValue::<PairingFlags>::from_bytes(&bytes)
            .map(|x| x.0)
            .unwrap_or_default();

        pair_setup_m1m2(&state, flags)
            .await
            .map(IntoResponse::into_response)
            .map_err(IntoResponse::into_response)
    } else {
        match M3Msg::from_bytes(&bytes) {
            Ok(TaggedValue(((), pubkey, proof))) => pair_setup_m3m4(&state, &pubkey, &proof)
                .await
                .map(IntoResponse::into_response)
                .map_err(IntoResponse::into_response),
            Err(err) => {
                // TODO: M5/M6?
                Err(err.into_response())
            }
        }
    }
}

async fn pair_setup_m1m2(
    state: &ServiceState,
    flags: PairingFlags,
) -> Result<M2Msg, ErrorResponse<state::M2>> {
    let (public_key, salt) = state
        .setup_state
        .lock()
        .await
        .m1_m2(rand::rng())
        .map_err(|err| TaggedValue(((), err)))?;

    Ok(TaggedValue(((), public_key, salt, flags)) as M2Msg)
}

async fn pair_setup_m3m4(
    state: &ServiceState,
    pubkey: &[u8],
    proof: &[u8],
) -> Result<M4Msg, ErrorResponse<state::M4>> {
    let proof = state
        .setup_state
        .lock()
        .await
        .m3_m4(pubkey, proof)
        .map_err(|err| TaggedValue(((), err)))?;

    Ok(TaggedValue(((), proof)))
}

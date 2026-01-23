use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::StatusCode;

use crate::config::Keychain;

use super::{
    dto::{
        EncryptedData, ErrorCode, Identifier, Method, PairingFlags, PairingState, Proof, PublicKey,
        Salt, Signature, method, state,
    },
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
type M5Msg = TaggedValue<(PairingState<state::M5>, EncryptedData)>;
type M5MsgSub = TaggedValue<(Identifier, PublicKey, Signature)>;
type M6MsgSub = TaggedValue<(Identifier, PublicKey, Signature)>;
type M6Msg = TaggedValue<(PairingState<state::M6>, EncryptedData)>;
type ErrorResponse<S> = TaggedValue<(PairingState<S>, ErrorCode)>;

pub async fn pair_setup<K>(
    State(state): State<Arc<ServiceState<K>>>,
    bytes: Bytes,
) -> Result<Response, Response>
where
    K: Keychain,
{
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
    } else if let Ok(TaggedValue(((), pubkey, proof))) = M3Msg::from_bytes(&bytes) {
        pair_setup_m3m4(&state, &pubkey, &proof)
            .await
            .map(IntoResponse::into_response)
            .map_err(IntoResponse::into_response)
    } else {
        match M5Msg::from_bytes(&bytes) {
            Ok(TaggedValue(((), mut enc_tlv))) => {
                pair_setup_m5m6_dec(&state, &mut enc_tlv)
                    .await
                    .map_err(IntoResponse::into_response)?;

                match M5MsgSub::from_bytes(&enc_tlv) {
                    Ok(TaggedValue((identifier, pubkey, signature))) => {
                        let sub_tlv = pair_setup_m5m6(&state, &identifier, &pubkey, &signature)
                            .await
                            .map_err(IntoResponse::into_response)?;
                        let msg = sub_tlv.bytes().collect::<Vec<u8>>();

                        pair_setup_m5m6_enc(&state, msg)
                            .await
                            .map(IntoResponse::into_response)
                            .map_err(IntoResponse::into_response)
                    }
                    Err(err) => Err(err.into_response()),
                }
            }
            Err(err) => Err(err.into_response()),
        }
    }
}

async fn pair_setup_m1m2<K>(
    state: &ServiceState<K>,
    flags: PairingFlags,
) -> Result<M2Msg, ErrorResponse<state::M2>> {
    state
        .setup_state
        .lock()
        .await
        .m1_m2(rand::rng())
        .map(|(pubkey, salt)| TaggedValue(((), pubkey, salt, flags)))
        .map_err(|err| TaggedValue(((), err)))
}

async fn pair_setup_m3m4<K>(
    state: &ServiceState<K>,
    pubkey: &[u8],
    proof: &[u8],
) -> Result<M4Msg, ErrorResponse<state::M4>> {
    state
        .setup_state
        .lock()
        .await
        .m3_m4(pubkey, proof)
        .map(|proof| TaggedValue(((), proof)))
        .map_err(|err| TaggedValue(((), err)))
}

async fn pair_setup_m5m6_dec<K>(
    state: &ServiceState<K>,
    enc_tlv: &mut Vec<u8>,
) -> Result<(), ErrorResponse<state::M5>> {
    state
        .setup_state
        .lock()
        .await
        .m5_m6_dec(enc_tlv)
        .map_err(|err| TaggedValue(((), err)))?;

    Ok(())
}

async fn pair_setup_m5m6<K>(
    state: &ServiceState<K>,
    device_id: &[u8],
    device_pubkey: &[u8],
    device_signature: &[u8],
) -> Result<M6MsgSub, ErrorResponse<state::M6>>
where
    K: Keychain,
{
    let inner = state.setup_state.lock().await;
    inner
        .m5_m6_verify(device_id, device_pubkey, device_signature)
        .map_err(|err| TaggedValue(((), err)))?;

    let keychain = state.keychain_holder.keychain();
    if !keychain.trust(device_id, device_pubkey) {
        return Err(TaggedValue(((), ErrorCode::Authentication)));
    }

    let accessory_id = keychain.id();
    let accessory_pubkey = keychain.pubkey();
    let accessory_signature = inner
        .m5_m6_generate_signature(accessory_id, accessory_pubkey, |msg| keychain.sign(msg))
        .map_err(|err| TaggedValue(((), err)))?;

    Ok(TaggedValue((
        accessory_id.to_vec(),
        accessory_pubkey.to_vec(),
        accessory_signature,
    )))
}

async fn pair_setup_m5m6_enc<K>(
    state: &ServiceState<K>,
    mut msg: Vec<u8>,
) -> Result<M6Msg, ErrorResponse<state::M6>> {
    state
        .setup_state
        .lock()
        .await
        .m5_m6_enc(&mut msg)
        .map(|_| TaggedValue(((), msg)))
        .map_err(|err| TaggedValue(((), err)))
}

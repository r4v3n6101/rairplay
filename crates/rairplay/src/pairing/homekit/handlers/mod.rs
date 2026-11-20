use std::sync::Arc;

use axum::{
    Extension,
    extract::State,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::StatusCode;
use yoke::{Yoke, erased::ErasedArcCart};

use super::{
    super::{SessionKey, SharedSessionKey},
    dto::{
        EncryptedData, ErrorCode, Identifier, Method, PairingFlags, PairingState, Proof, PublicKey,
        Salt, Signature, method, state,
    },
    extractor::TaggedValue,
    state::ServiceState,
};
use crate::config::Keychain;

pub mod setup;
pub mod verify;

/* Pair setup types */

// PairingFlags are optional
type PSM1Msg = TaggedValue<(PairingState<state::M1>, Method<method::PairSetup>)>;
type PSM2Msg = TaggedValue<(PairingState<state::M2>, PublicKey, Salt, PairingFlags)>;
type PSM3Msg = TaggedValue<(PairingState<state::M3>, PublicKey, Proof)>;
type PSM4Msg = TaggedValue<(PairingState<state::M4>, Proof)>;
type PSM5Msg = TaggedValue<(PairingState<state::M5>, EncryptedData)>;
type PSM5MsgSub = TaggedValue<(Identifier, PublicKey, Signature)>;
type PSM6MsgSub = TaggedValue<(Identifier, PublicKey, Signature)>;
type PSM6Msg = TaggedValue<(PairingState<state::M6>, EncryptedData)>;

/* Pair verify types */

type PVM1Msg = TaggedValue<(PairingState<state::M1>, PublicKey)>;
type PVM2MsbSub = TaggedValue<(Identifier, Signature)>;
type PVM2Msg = TaggedValue<(PairingState<state::M2>, PublicKey, EncryptedData)>;
type PVM3Msg = TaggedValue<(PairingState<state::M3>, EncryptedData)>;
type PVM3MsgSub = TaggedValue<(Identifier, Signature)>;
type PVM4Msg = TaggedValue<PairingState<state::M4>>;

type ErrorResponse<S> = TaggedValue<(PairingState<S>, ErrorCode)>;

pub async fn pair_setup<K>(
    State(state): State<Arc<ServiceState>>,
    Extension(keychain): Extension<Yoke<&'static K, ErasedArcCart>>,
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

    if PSM1Msg::from_bytes(&bytes).is_ok() {
        let flags = TaggedValue::<PairingFlags>::from_bytes(&bytes)
            .map(|x| x.0)
            .unwrap_or_default();

        Ok(pair_setup_m1m2(&state, flags).into_response())
    } else if let Ok(TaggedValue(((), pubkey, proof))) = PSM3Msg::from_bytes(&bytes) {
        pair_setup_m3m4(&state, &pubkey, &proof)
            .map(IntoResponse::into_response)
            .map_err(IntoResponse::into_response)
    } else {
        match PSM5Msg::from_bytes(&bytes) {
            Ok(TaggedValue(((), mut enc_tlv))) => {
                pair_setup_m5m6_dec(&state, &mut enc_tlv).map_err(IntoResponse::into_response)?;

                match PSM5MsgSub::from_bytes(&enc_tlv) {
                    Ok(TaggedValue((identifier, pubkey, signature))) => {
                        let sub_tlv = pair_setup_m5m6(
                            &state,
                            *keychain.get(),
                            &identifier,
                            &pubkey,
                            &signature,
                        )
                        .map_err(IntoResponse::into_response)?;
                        let msg = sub_tlv.bytes().collect::<Vec<u8>>();

                        pair_setup_m5m6_enc(&state, msg)
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

pub async fn pair_verify<K>(
    State(state): State<Arc<ServiceState>>,
    Extension(keychain): Extension<Yoke<&'static K, ErasedArcCart>>,
    Extension(session_key): Extension<SharedSessionKey>,
    bytes: Bytes,
) -> Result<Response, Response>
where
    K: Keychain,
{
    if let Ok(TaggedValue(((), pubkey))) = PVM1Msg::from_bytes(&bytes) {
        let (accessory_tmp_pubkey, sub_tlv) = pair_verify_m1m2(&state, *keychain.get(), &pubkey)
            .map_err(IntoResponse::into_response)?;
        let msg = sub_tlv.bytes().collect::<Vec<u8>>();

        pair_verify_m1m2_enc(&state, accessory_tmp_pubkey, msg)
            .map(IntoResponse::into_response)
            .map_err(IntoResponse::into_response)
    } else {
        match PVM3Msg::from_bytes(&bytes) {
            Ok(TaggedValue(((), mut enc_tlv))) => {
                pair_verify_m3m4_dec(&state, &mut enc_tlv).map_err(IntoResponse::into_response)?;

                match PVM3MsgSub::from_bytes(&enc_tlv) {
                    Ok(TaggedValue((device_id, device_signature))) => pair_verify_m3m4(
                        &state,
                        &session_key,
                        *keychain.get(),
                        &device_id,
                        &device_signature,
                    )
                    .map(IntoResponse::into_response)
                    .map_err(IntoResponse::into_response),
                    Err(err) => Err(err.into_response()),
                }
            }
            Err(err) => Err(err.into_response()),
        }
    }
}

fn pair_setup_m1m2(state: &ServiceState, flags: PairingFlags) -> PSM2Msg {
    let (pubkey, salt) = state.setup_state.lock().unwrap().m1_m2(rand::rng());
    TaggedValue(((), pubkey, salt, flags))
}

fn pair_setup_m3m4(
    state: &ServiceState,
    pubkey: &[u8],
    proof: &[u8],
) -> Result<PSM4Msg, ErrorResponse<state::M4>> {
    state
        .setup_state
        .lock()
        .unwrap()
        .m3_m4(pubkey, proof)
        .map(|proof| TaggedValue(((), proof)))
        .map_err(|err| TaggedValue(((), err)))
}

fn pair_setup_m5m6_dec(
    state: &ServiceState,
    enc_tlv: &mut Vec<u8>,
) -> Result<(), ErrorResponse<state::M5>> {
    state
        .setup_state
        .lock()
        .unwrap()
        .m5_m6_dec(enc_tlv)
        .map_err(|err| TaggedValue(((), err)))
}

fn pair_setup_m5m6<K>(
    state: &ServiceState,
    keychain: &K,
    device_id: &[u8],
    device_pubkey: &[u8],
    device_signature: &[u8],
) -> Result<PSM6MsgSub, ErrorResponse<state::M6>>
where
    K: Keychain,
{
    let inner = state.setup_state.lock().unwrap();
    inner
        .m5_m6_verify(device_id, device_pubkey, device_signature)
        .map_err(|err| TaggedValue(((), err)))?;

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

fn pair_setup_m5m6_enc(
    state: &ServiceState,
    mut msg: Vec<u8>,
) -> Result<PSM6Msg, ErrorResponse<state::M6>> {
    state
        .setup_state
        .lock()
        .unwrap()
        .m5_m6_enc(&mut msg)
        .map(|()| TaggedValue(((), msg)))
        .map_err(|err| TaggedValue(((), err)))
}

fn pair_verify_m1m2<K>(
    state: &ServiceState,
    keychain: &K,
    device_pubkey: &[u8],
) -> Result<(Vec<u8>, PVM2MsbSub), ErrorResponse<state::M1>>
where
    K: Keychain,
{
    let mut inner = state.verify_state.lock().unwrap();
    let identity = keychain.id();
    inner
        .m1_m2(rand::rng(), device_pubkey, identity, |msg| {
            keychain.sign(msg)
        })
        .map(|(accessory_tmp_pubkey, accessory_signature)| {
            (
                accessory_tmp_pubkey,
                TaggedValue((identity.to_vec(), accessory_signature)),
            )
        })
        .map_err(|err| TaggedValue(((), err)))
}

fn pair_verify_m1m2_enc(
    state: &ServiceState,
    accessory_tmp_pubkey: Vec<u8>,
    mut msg: Vec<u8>,
) -> Result<PVM2Msg, ErrorResponse<state::M1>> {
    state
        .verify_state
        .lock()
        .unwrap()
        .m1_m2_enc(&mut msg)
        .map(|()| TaggedValue(((), accessory_tmp_pubkey, msg)))
        .map_err(|err| TaggedValue(((), err)))
}

fn pair_verify_m3m4_dec(
    state: &ServiceState,
    enc_tlv: &mut Vec<u8>,
) -> Result<(), ErrorResponse<state::M3>> {
    state
        .verify_state
        .lock()
        .unwrap()
        .m3_m4_dec(enc_tlv)
        .map_err(|err| TaggedValue(((), err)))
}

fn pair_verify_m3m4<K>(
    state: &ServiceState,
    session_key: &SharedSessionKey,
    keychain: &K,
    device_id: &[u8],
    device_signature: &[u8],
) -> Result<PVM4Msg, ErrorResponse<state::M3>>
where
    K: Keychain,
{
    let inner = state.verify_state.lock().unwrap();
    inner
        .m3_m4(device_id, device_signature, |msg, signature| {
            keychain.verify(device_id, msg, signature)
        })
        .inspect(|&shared_secret| {
            session_key.lock_write().replace(SessionKey {
                key_material: shared_secret,
                upgrade_channel: true,
            });
        })
        .map(|_| TaggedValue(()))
        .map_err(|err| TaggedValue(((), err)))
}

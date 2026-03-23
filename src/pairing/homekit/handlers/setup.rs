use std::borrow::Cow;

use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce, aead::AeadInOut};
use ed25519_dalek::{Signature, VerifyingKey};
use rand::{Rng, RngExt};
use sha2::Sha512;
use srp::{ClientG3072, ServerG3072};

use super::super::dto::ErrorCode;
use crate::{config::PinCode, crypto::hkdf};

type SaltArray = [u8; 16];
type PrivKeyArray = [u8; 64];

enum Inner {
    Init,
    AuthStart {
        salt: SaltArray,
        privkey: PrivKeyArray,
        verifier: Vec<u8>,
    },
    Transient {
        session_key: Vec<u8>,
    },
}

pub struct State {
    username: &'static str,
    password: Cow<'static, str>,
    inner: Inner,
}

impl State {
    pub fn new(pin: Option<PinCode>) -> Self {
        const PAIR_SETUP_USERNAME: &str = "Pair-Setup";
        const PAIR_SETUP_DEFAULT_PASSWORD: &str = "3939";

        let username = PAIR_SETUP_USERNAME;
        let password = if let Some(pin) = pin {
            Cow::Owned(format!("{pin}"))
        } else {
            Cow::Borrowed(PAIR_SETUP_DEFAULT_PASSWORD)
        };

        Self {
            username,
            password,
            inner: Inner::Init,
        }
    }

    pub fn m1_m2(&mut self, mut rand: impl Rng) -> (Vec<u8>, Vec<u8>) {
        let salt: SaltArray = rand.random();
        let privkey: PrivKeyArray = rand.random();

        let srp_client = ClientG3072::<Sha512>::new_with_options(true);
        let verifier =
            srp_client.compute_verifier(self.username.as_bytes(), self.password.as_bytes(), &salt);

        let srp_server = ServerG3072::<Sha512>::new();
        let pubkey = srp_server.compute_public_ephemeral(&privkey, &verifier);

        self.inner = Inner::AuthStart {
            salt,
            privkey,
            verifier,
        };

        (pubkey, salt.to_vec())
    }

    pub fn m3_m4(
        &mut self,
        client_pubkey: &[u8],
        client_proof: &[u8],
    ) -> Result<Vec<u8>, ErrorCode> {
        let Inner::AuthStart {
            salt,
            privkey,
            verifier,
        } = &self.inner
        else {
            return Err(ErrorCode::Busy);
        };

        let srp_server = ServerG3072::<Sha512>::new_with_options(true);

        let Ok(reply) = srp_server.process_reply(
            self.username.as_bytes(),
            salt,
            privkey,
            verifier,
            client_pubkey,
        ) else {
            return Err(ErrorCode::Authentication);
        };

        let Ok(session_key) = reply.verify_client(client_proof) else {
            return Err(ErrorCode::Authentication);
        };

        self.inner = Inner::Transient {
            session_key: session_key.to_vec(),
        };

        Ok(reply.proof().to_vec())
    }

    pub fn m5_m6_dec(&self, msg: &mut Vec<u8>) -> Result<(), ErrorCode> {
        const NONCE: &[u8] = b"\0\0\0\0PS-Msg05";
        const SALT: &[u8] = b"Pair-Setup-Encrypt-Salt";
        const INFO: &[u8] = b"Pair-Setup-Encrypt-Info";

        let Inner::Transient { session_key } = &self.inner else {
            return Err(ErrorCode::Busy);
        };

        let session_key = hkdf(session_key, SALT, INFO);
        let cipher = ChaCha20Poly1305::new(&session_key.into());
        if cipher
            .decrypt_in_place(&Nonce::try_from(NONCE).unwrap(), &[], msg)
            .is_err()
        {
            return Err(ErrorCode::Authentication);
        }

        Ok(())
    }

    pub fn m5_m6_verify(
        &self,
        device_id: &[u8],
        device_pubkey: &[u8],
        device_signature: &[u8],
    ) -> Result<(), ErrorCode> {
        const SALT: &[u8] = b"Pair-Setup-Controller-Sign-Salt";
        const INFO: &[u8] = b"Pair-Setup-Controller-Sign-Info";

        let Inner::Transient { session_key } = &self.inner else {
            return Err(ErrorCode::Busy);
        };

        let device_x = hkdf(session_key, SALT, INFO);
        let mut device_info =
            Vec::with_capacity(device_x.len() + device_id.len() + device_pubkey.len());
        device_info.extend_from_slice(&device_x);
        device_info.extend_from_slice(device_id);
        device_info.extend_from_slice(device_pubkey);

        let Ok(pubkey) = device_pubkey.try_into() else {
            return Err(ErrorCode::Authentication);
        };
        let Ok(verifying_key) = VerifyingKey::from_bytes(&pubkey) else {
            return Err(ErrorCode::Authentication);
        };
        let Ok(signature) = Signature::from_slice(device_signature) else {
            return Err(ErrorCode::Authentication);
        };
        if verifying_key
            .verify_strict(&device_info, &signature)
            .is_err()
        {
            return Err(ErrorCode::Authentication);
        }

        Ok(())
    }

    pub fn m5_m6_generate_signature<F>(
        &self,
        accessory_id: &[u8],
        accessory_pubkey: &[u8],
        sign: F,
    ) -> Result<Vec<u8>, ErrorCode>
    where
        F: FnOnce(&[u8]) -> Vec<u8>,
    {
        const SALT: &[u8] = b"Pair-Setup-Accessory-Sign-Salt";
        const INFO: &[u8] = b"Pair-Setup-Accessory-Sign-Info";

        let Inner::Transient { session_key } = &self.inner else {
            return Err(ErrorCode::Busy);
        };

        let accessory_x = hkdf(session_key, SALT, INFO);
        let mut accessory_info =
            Vec::with_capacity(accessory_x.len() + accessory_id.len() + accessory_pubkey.len());
        accessory_info.extend_from_slice(&accessory_x);
        accessory_info.extend_from_slice(accessory_id);
        accessory_info.extend_from_slice(accessory_pubkey);

        Ok(sign(&accessory_info))
    }

    pub fn m5_m6_enc(&self, msg: &mut Vec<u8>) -> Result<(), ErrorCode> {
        const NONCE: &[u8] = b"\0\0\0\0PS-Msg06";
        const SALT: &[u8] = b"Pair-Setup-Encrypt-Salt";
        const INFO: &[u8] = b"Pair-Setup-Encrypt-Info";

        let Inner::Transient { session_key } = &self.inner else {
            return Err(ErrorCode::Busy);
        };

        let session_key = hkdf(session_key, SALT, INFO);
        let cipher = ChaCha20Poly1305::new(&session_key.into());
        if cipher
            .encrypt_in_place(&Nonce::try_from(NONCE).unwrap(), &[], msg)
            .is_err()
        {
            return Err(ErrorCode::Authentication);
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(non_upper_case_globals, dead_code)]
mod tests {
    use super::*;

    const s: [u8; 16] = [
        182, 14, 130, 190, 42, 45, 109, 155, 242, 253, 216, 201, 155, 43, 162, 148,
    ];
    const v: &[u8] = &[
        55, 37, 252, 232, 231, 66, 221, 93, 148, 11, 87, 53, 247, 244, 238, 115, 178, 60, 252, 238,
        200, 76, 43, 159, 135, 61, 43, 63, 254, 108, 245, 92, 64, 41, 183, 229, 4, 10, 200, 54, 6,
        73, 243, 117, 170, 241, 121, 52, 63, 148, 39, 227, 223, 6, 244, 181, 32, 189, 90, 143, 138,
        55, 125, 15, 241, 33, 179, 153, 67, 72, 15, 245, 216, 37, 46, 47, 35, 84, 167, 56, 192, 49,
        79, 205, 249, 253, 63, 8, 134, 108, 95, 175, 72, 100, 245, 10, 83, 255, 244, 57, 96, 7, 63,
        18, 25, 37, 234, 120, 219, 211, 52, 111, 231, 191, 38, 232, 217, 231, 15, 247, 115, 232,
        113, 13, 37, 40, 178, 119, 147, 31, 163, 75, 79, 142, 34, 8, 197, 185, 228, 84, 235, 193,
        142, 62, 49, 184, 253, 167, 140, 212, 26, 57, 215, 211, 96, 130, 160, 3, 2, 31, 159, 235,
        253, 65, 179, 223, 217, 100, 228, 49, 210, 133, 136, 161, 222, 57, 119, 110, 37, 90, 165,
        227, 136, 107, 57, 94, 188, 32, 49, 14, 66, 125, 249, 28, 75, 186, 68, 223, 174, 13, 134,
        81, 232, 15, 79, 128, 171, 89, 218, 12, 98, 176, 66, 0, 236, 175, 161, 232, 157, 119, 53,
        249, 193, 37, 154, 25, 166, 218, 15, 95, 237, 175, 132, 251, 1, 255, 36, 168, 2, 48, 43,
        99, 100, 80, 126, 223, 205, 194, 212, 146, 221, 160, 115, 204, 55, 42, 90, 248, 220, 104,
        106, 5, 116, 68, 189, 123, 167, 208, 186, 211, 70, 212, 17, 31, 177, 8, 165, 53, 60, 108,
        220, 253, 176, 199, 80, 205, 145, 150, 67, 25, 182, 148, 53, 250, 166, 242, 166, 219, 141,
        119, 25, 109, 236, 246, 34, 193, 141, 7, 254, 17, 126, 139, 25, 124, 166, 61, 201, 171,
        185, 223, 252, 67, 191, 3, 225, 44, 180, 210, 180, 2, 70, 101, 176, 184, 175, 197, 11, 123,
        167, 78, 202, 219, 98, 93, 80, 13, 61, 53, 53, 106, 82, 167, 7, 58, 175, 206, 92, 143, 137,
        150, 19, 164, 160, 241, 23, 46, 153, 194, 77, 146, 107, 120, 133, 7, 117, 158, 59, 195,
        116, 13, 176, 213, 10, 121,
    ];
    const b: [u8; 64] = [
        191, 85, 237, 93, 149, 43, 112, 166, 154, 159, 214, 74, 28, 72, 248, 254, 140, 164, 163,
        175, 196, 197, 120, 2, 124, 22, 6, 11, 130, 171, 57, 77, 184, 107, 45, 193, 193, 234, 209,
        71, 157, 252, 170, 174, 180, 218, 150, 102, 49, 164, 55, 78, 79, 247, 110, 209, 21, 128,
        244, 67, 164, 199, 67, 222,
    ];
    const B: &[u8] = &[
        168, 110, 137, 254, 225, 245, 29, 126, 125, 182, 43, 133, 19, 20, 78, 193, 78, 184, 10, 47,
        69, 166, 142, 200, 198, 255, 49, 251, 234, 218, 22, 207, 163, 134, 233, 129, 105, 38, 111,
        233, 127, 186, 221, 203, 134, 110, 207, 147, 143, 221, 48, 237, 55, 193, 200, 12, 124, 74,
        14, 193, 251, 56, 5, 254, 37, 54, 251, 187, 224, 55, 151, 87, 21, 238, 227, 192, 152, 115,
        163, 18, 139, 16, 31, 99, 220, 242, 32, 130, 177, 216, 217, 226, 178, 233, 182, 172, 177,
        185, 235, 126, 12, 97, 42, 127, 126, 71, 99, 68, 10, 206, 126, 223, 19, 214, 161, 136, 194,
        159, 236, 0, 116, 78, 244, 189, 222, 201, 129, 242, 249, 85, 182, 19, 101, 10, 80, 33, 76,
        112, 19, 98, 178, 204, 120, 118, 135, 26, 175, 205, 212, 168, 135, 203, 74, 101, 133, 176,
        57, 218, 231, 79, 191, 254, 0, 216, 188, 107, 213, 99, 17, 183, 139, 34, 243, 231, 57, 82,
        137, 77, 154, 161, 232, 18, 73, 110, 30, 163, 230, 193, 36, 192, 38, 128, 8, 35, 90, 82,
        35, 124, 161, 41, 70, 168, 203, 98, 16, 214, 28, 140, 146, 181, 246, 202, 24, 34, 229, 56,
        182, 194, 101, 8, 223, 239, 240, 130, 73, 222, 146, 238, 224, 58, 191, 244, 24, 176, 183,
        160, 100, 207, 3, 30, 23, 183, 44, 159, 252, 158, 172, 180, 166, 161, 201, 209, 183, 141,
        88, 39, 190, 216, 213, 14, 169, 215, 6, 91, 166, 131, 125, 52, 246, 206, 71, 209, 74, 104,
        2, 205, 240, 150, 180, 139, 219, 107, 5, 74, 104, 42, 66, 193, 145, 236, 244, 77, 117, 160,
        48, 153, 45, 125, 193, 50, 205, 194, 64, 195, 223, 234, 109, 101, 179, 37, 150, 200, 24,
        183, 217, 76, 26, 70, 112, 118, 23, 134, 96, 103, 14, 106, 129, 162, 226, 239, 210, 94,
        123, 221, 249, 56, 86, 64, 111, 90, 224, 53, 92, 185, 187, 38, 88, 197, 118, 230, 48, 142,
        179, 8, 183, 205, 244, 139, 156, 48, 233, 55, 177, 65, 121, 125, 180, 206, 234, 100, 205,
        205, 215, 0, 248, 20, 212, 218, 129, 164, 141, 228, 90, 70,
    ];
    const A: &[u8] = &[
        66, 83, 212, 246, 87, 0, 80, 45, 236, 35, 245, 239, 166, 200, 90, 42, 16, 179, 28, 95, 227,
        65, 93, 178, 170, 173, 133, 192, 20, 133, 212, 250, 59, 48, 22, 16, 236, 203, 169, 137, 9,
        119, 162, 150, 35, 138, 129, 177, 86, 59, 13, 49, 106, 134, 156, 163, 124, 18, 156, 254,
        49, 8, 163, 249, 152, 223, 118, 214, 35, 190, 229, 176, 154, 7, 204, 179, 85, 78, 253, 190,
        2, 17, 109, 96, 79, 164, 118, 137, 227, 112, 38, 42, 164, 207, 165, 205, 255, 221, 247,
        168, 19, 137, 186, 13, 62, 217, 107, 56, 216, 194, 157, 167, 81, 255, 82, 123, 101, 207,
        72, 215, 210, 159, 183, 156, 17, 167, 46, 79, 16, 173, 125, 27, 42, 113, 175, 62, 35, 195,
        175, 239, 153, 223, 102, 192, 46, 238, 252, 228, 74, 207, 243, 32, 25, 217, 187, 249, 33,
        92, 19, 57, 225, 71, 160, 207, 178, 0, 6, 185, 116, 136, 35, 229, 251, 111, 133, 190, 165,
        94, 7, 65, 203, 148, 238, 102, 63, 215, 221, 199, 158, 124, 107, 69, 11, 89, 164, 248, 195,
        66, 144, 154, 186, 160, 228, 5, 178, 191, 32, 135, 92, 55, 134, 145, 24, 11, 63, 214, 61,
        98, 4, 189, 134, 65, 119, 173, 177, 237, 40, 61, 191, 24, 133, 108, 163, 242, 121, 229,
        213, 118, 174, 88, 194, 116, 14, 218, 13, 217, 16, 75, 1, 65, 80, 170, 165, 185, 100, 60,
        101, 58, 153, 133, 89, 242, 199, 203, 225, 86, 88, 85, 113, 112, 83, 192, 100, 95, 223, 78,
        176, 68, 161, 62, 134, 61, 51, 209, 25, 67, 55, 64, 174, 205, 27, 187, 38, 209, 225, 67,
        169, 36, 121, 43, 179, 50, 162, 57, 34, 101, 226, 221, 10, 149, 108, 3, 35, 218, 237, 250,
        20, 198, 123, 107, 157, 121, 97, 188, 208, 18, 136, 243, 230, 164, 157, 160, 57, 170, 94,
        125, 126, 153, 87, 41, 163, 233, 240, 37, 33, 214, 103, 18, 31, 25, 143, 223, 244, 39, 51,
        202, 117, 159, 76, 74, 76, 3, 45, 99, 18, 29, 9, 73, 122, 0, 70, 65, 229, 221, 99, 203, 22,
        169, 124, 26, 28, 227, 38, 176,
    ];
    const K: &[u8] = &[
        157, 15, 67, 105, 217, 231, 94, 131, 93, 166, 167, 201, 39, 91, 61, 114, 21, 18, 5, 55,
        250, 33, 215, 168, 242, 143, 135, 226, 70, 162, 76, 166, 4, 87, 107, 250, 222, 72, 53, 199,
        243, 73, 229, 111, 29, 32, 116, 236, 254, 183, 175, 128, 148, 167, 111, 141, 138, 81, 238,
        194, 134, 70, 156, 117,
    ];
    const M1: &[u8] = &[
        246, 111, 83, 159, 179, 26, 220, 113, 155, 160, 160, 147, 242, 191, 109, 154, 214, 41, 64,
        245, 10, 62, 10, 67, 193, 23, 170, 229, 25, 135, 90, 53, 213, 138, 218, 231, 122, 188, 71,
        160, 64, 197, 209, 95, 198, 57, 223, 183, 116, 251, 125, 29, 141, 223, 25, 235, 70, 51, 73,
        249, 234, 104, 252, 133,
    ];
    const M2: &[u8] = &[
        174, 254, 88, 248, 151, 155, 238, 151, 37, 251, 118, 35, 37, 126, 76, 208, 64, 70, 8, 94,
        141, 207, 164, 126, 159, 241, 224, 21, 218, 205, 57, 68, 235, 158, 91, 4, 223, 18, 207,
        139, 98, 55, 95, 93, 182, 201, 169, 182, 251, 193, 163, 246, 37, 157, 190, 166, 236, 22,
        164, 39, 232, 140, 178, 94,
    ];

    #[test]
    fn test_client_proof() {
        let srp_server = ServerG3072::<Sha512>::new_with_options(true);
        let reply = srp_server
            .process_reply(b"Pair-Setup", &s, &b, v, A)
            .unwrap();

        let session_key = reply.verify_client(M1).unwrap();
        assert_eq!(M2, reply.proof());
        assert_eq!(K, session_key);
    }
}

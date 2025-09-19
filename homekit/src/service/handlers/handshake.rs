use std::mem;

use rand::{CryptoRng, Rng};
use sha2::Sha512;
use srp::{client::SrpClient, groups::G_3072};

type Salt = [u8; 16];
type Privkey = [u8; 64];

pub enum Response {
    Error { state: StateCode, error: ErrorCode },
    M2 { salt: Salt, public_key: Vec<u8> },
    M4 { proof: Vec<u8> },
}

pub enum State {
    Invalid,
    Initialized {
        username: &'static [u8],
        password: PinCode,
    },
    M12 {
        username: &'static [u8],
        password: PinCode,
        salt: Salt,
        secret_key: Privkey,
    },
    M34 {
        shared_secret: Vec<u8>,
    },
    // TODO: non-transient with encryption of tcp channel
}

impl State {
    // TODO : change pin to concreate type
    pub fn initialize(username: &'static [u8], password: PinCode) -> Self {
        Self::Initialized { username, password }
    }

    pub fn m1_m2(&mut self, mut rand: impl CryptoRng) -> Response {
        let Self::Initialized { username, password } = mem::replace(self, Self::Invalid) else {
            return Response::Error {
                state: StateCode::M2,
                error: todo!(),
            };
        };

        let secret_key: Privkey = rand.random();
        let salt: Salt = rand.random();

        let srp_client = SrpClient::<Sha512>::new(&G_3072);
        let public_key = srp_client.compute_public_ephemeral(&secret_key);

        *self = Self::M12 {
            username,
            password,
            salt,
            secret_key,
        };

        Response::M2 { salt, public_key }
    }

    // TODO : change inputs
    pub fn m3_m4(&mut self, client_pubkey: &[u8], client_proof: &[u8]) -> Response {
        let Self::M12 {
            username,
            password,
            salt,
            secret_key,
        } = mem::replace(self, Self::Invalid)
        else {
            // TODO: send tlv8 error
            // return Err((tlv8::State::M4, WTF));
            return Response::Error {
                state: StateCode::M4,
                error: todo!(),
            };
        };
        let srp_client = SrpClient::<Sha512>::new(&G_3072);

        let Ok(res) = srp_client.process_reply(
            &secret_key,
            username,
            &password.to_bytes(),
            &salt,
            client_pubkey,
        ) else {
            return Response::Error {
                state: StateCode::M4,
                error: ErrorCode::Authentication,
            };
        };

        if res.verify_server(client_proof).is_err() {
            return Response::Error {
                state: StateCode::M4,
                error: ErrorCode::Authentication,
            };
        }

        *self = Self::M34 {
            shared_secret: res.key().to_vec(),
        };

        Response::M4 {
            proof: res.proof().to_vec(),
        }
    }

    pub fn shared_secret(self) -> Option<Vec<u8>> {
        match self {
            Self::M34 { shared_secret } => Some(shared_secret),
            _ => None,
        }
    }
}

pub struct State2<T> {
    pub attempts: usize,
    pub pin: PinCode,
    pub state: T,
}

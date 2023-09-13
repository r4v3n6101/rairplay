use std::{io, net::IpAddr};

use lazy_static::lazy_static;
use mac_address2::MacAddress;
use openssl::{
    aes::AesKey,
    base64,
    pkey::Private,
    rsa::{Padding, Rsa},
};

lazy_static! {
    // TODO : remove openssl as dependency
    static ref AIRPORT_PRIVATE_KEY: Rsa<Private> =
        Rsa::private_key_from_pem(include_bytes!("airport.key")).expect("valid PEM file");
}

#[derive(Debug)]
pub enum KeyEncryption {
    Rsa { aeskey64: String, aesiv64: String },
    FairPlay {/* TODO */},
}

// TODO : #[derive(Debug)]
pub struct AesKeyIv {
    pub key: AesKey,
    pub iv: Vec<u8>,
}

impl KeyEncryption {
    pub fn decode(self) -> io::Result<AesKeyIv> {
        match self {
            KeyEncryption::Rsa { aeskey64, aesiv64 } => {
                let aeskey = base64::decode_block(&aeskey64)?;
                let aesiv = base64::decode_block(&aesiv64)?;

                let mut aeskey_to = vec![0; 256];
                AIRPORT_PRIVATE_KEY.private_decrypt(&aeskey, &mut aeskey_to, Padding::PKCS1)?;

                let aeskey = AesKey::new_decrypt(&aeskey_to).map_err(|_| {
                    io::Error::new(io::ErrorKind::Other, "couldn't create decrypt key")
                })?;
                Ok(AesKeyIv {
                    key: aeskey,
                    iv: aesiv,
                })
            }
            _ => unimplemented!(),
        }
    }
}

pub fn auth_with_challenge(
    challenge64: &str,
    addr: &IpAddr,
    mac_addr: &MacAddress,
) -> io::Result<String> {
    let challenge = base64::decode_block(challenge64)?;

    let mut message = Vec::with_capacity(48);
    message.extend_from_slice(&challenge);
    match addr {
        IpAddr::V4(addr) => message.extend_from_slice(&addr.octets()),
        IpAddr::V6(addr) => message.extend_from_slice(&addr.octets()),
    }
    message.extend_from_slice(&mac_addr.bytes());
    if message.len() < 32 {
        message.resize(32, 0);
    }

    let mut to = [0; 256];
    AIRPORT_PRIVATE_KEY.private_encrypt(&message, &mut to, Padding::PKCS1)?;

    Ok(base64::encode_block(&to))
}

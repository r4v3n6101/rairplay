use std::{io, net::Ipv4Addr};

use lazy_static::lazy_static;
use openssl::{
    base64,
    pkey::Private,
    rsa::{Padding, Rsa},
};

lazy_static! {
    // TODO : remove openssl as dependency
    static ref AIRPORT_PRIVATE_KEY: Rsa<Private> =
        Rsa::private_key_from_pem(include_bytes!("airport.key")).expect("valid PEM file");
}

pub fn auth_with_challenge(challenge: &str) -> io::Result<String> {
    let challenge = base64::decode_block(challenge)?;

    let mut message = Vec::with_capacity(48);
    message.extend_from_slice(&challenge);
    // TODO : replace w/ non-constant
    message.extend_from_slice(&Ipv4Addr::new(172, 20, 10, 6).octets());
    message.extend_from_slice(&[0xa0, 0xdb, 0x0c, 0x69, 0xd3, 0x6f]);
    if message.len() < 32 {
        message.resize(32, 0);
    }

    let mut to = [0; 256];
    AIRPORT_PRIVATE_KEY.private_encrypt(&message, &mut to, Padding::PKCS1)?;

    Ok(base64::encode_block(&to))
}

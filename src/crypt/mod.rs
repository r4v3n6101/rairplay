use std::{io, net::IpAddr};

use lazy_static::lazy_static;
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

pub fn auth_with_challenge(
    challenge64: &str,
    addr: &IpAddr,
    mac_addr: impl AsRef<[u8; 6]>,
) -> io::Result<String> {
    let challenge = base64::decode_block(challenge64)?;

    let mut message = Vec::with_capacity(48);
    message.extend_from_slice(&challenge);
    match addr {
        IpAddr::V4(addr) => message.extend_from_slice(&addr.octets()),
        IpAddr::V6(addr) => message.extend_from_slice(&addr.octets()),
    }
    message.extend_from_slice(mac_addr.as_ref());
    if message.len() < 32 {
        message.resize(32, 0);
    }

    let mut to = [0; 256];
    AIRPORT_PRIVATE_KEY.private_encrypt(&message, &mut to, Padding::PKCS1)?;

    Ok(base64::encode_block(&to))
}

pub fn rsaaeskey(rsaaeskey64: &str, aesiv64: &str) -> io::Result<(AesKey, Vec<u8>)> {
    let aeskey = base64::decode_block(rsaaeskey64)?;
    let aesiv = base64::decode_block(aesiv64)?;

    let mut aeskey_to = vec![0; 256];
    AIRPORT_PRIVATE_KEY.private_decrypt(&aeskey, &mut aeskey_to, Padding::PKCS1)?;

    Ok((
        AesKey::new_decrypt(&aeskey_to)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "couldn't create decrypt key"))?,
        aesiv,
    ))
}

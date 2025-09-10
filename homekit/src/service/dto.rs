use strum::FromRepr;

pub type SaltValue = [u8; 16];

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, FromRepr)]
pub enum TagCode {
    Method = 0,
    Identifier = 1,
    Salt = 2,
    PublicKey = 3,
    Proof = 4,
    EncryptedData = 5,
    State = 6,
    Error = 7,
    RetryDelay = 8,
    Certificate = 9,
    Signature = 10,
    Permissions = 11,
    FragmentData = 12,
    FragmentLast = 13,
    Flags = 19,
    Separator = 255,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, FromRepr)]
pub enum StateCode {
    M1 = 1,
    M2 = 2,
    M3 = 3,
    M4 = 4,
    M5 = 5,
    M6 = 6,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, FromRepr)]
pub enum MethodCode {
    PairSetup = 0,
    PairSetupAuth = 1,
    PairVerify = 2,
    AddPairing = 3,
    RemovePairing = 4,
    ListPairings = 5,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, FromRepr)]
pub enum ErrorCode {
    Reserved = 0,
    Unknown = 1,
    Authentication = 2,
    Backoff = 3,
    MaxPeers = 4,
    MaxTries = 5,
    Unavailable = 6,
    Busy = 7,
}

pub trait Tlv8 {
    const TAG: TagCode;
    type Value;

    fn length(value: &Self::Value) -> usize;
}

pub trait Tlv8Pack {
    type Value;
}

impl<T: Tlv8> Tlv8Pack for T {
    type Value = T::Value;
}

macro_rules! impl_tlv8_pack {
    ( $( $name:ident )+ ) => {
        impl<$($name: Tlv8),+> Tlv8Pack for ( $($name,)+ ) {
            type Value = ( $($name::Value,)+ );
        }
    };
}

impl_tlv8_pack!(T1 T2);
impl_tlv8_pack!(T1 T2 T3);
impl_tlv8_pack!(T1 T2 T3 T4);
impl_tlv8_pack!(T1 T2 T3 T4 T5);
impl_tlv8_pack!(T1 T2 T3 T4 T5 T6);
impl_tlv8_pack!(T1 T2 T3 T4 T5 T6 T7);
impl_tlv8_pack!(T1 T2 T3 T4 T5 T6 T7 T8);
impl_tlv8_pack!(T1 T2 T3 T4 T5 T6 T7 T8 T9);
impl_tlv8_pack!(T1 T2 T3 T4 T5 T6 T7 T8 T9 T10);

macro_rules! impl_tlv8 {
    ($name: ident, $value: ty, $length: expr) => {
        pub struct $name;
        impl Tlv8 for $name {
            const TAG: TagCode = TagCode::$name;
            type Value = $value;

            fn length(value: &Self::Value) -> usize {
                ($length)(value)
            }
        }
    };
}

impl_tlv8!(Method, MethodCode, |_| 1);
impl_tlv8!(Salt, SaltValue, |v: &[u8]| v.len());
impl_tlv8!(PublicKey, Vec<u8>, |v: &[u8]| v.len());

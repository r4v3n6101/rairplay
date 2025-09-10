use std::mem;

use bitflags::bitflags;
use strum::{Display, FromRepr};

#[repr(u8)]
#[derive(Display, Debug, Copy, Clone, PartialEq, Eq, FromRepr)]
pub enum TagCode {
    Method = 0,
    Identifier = 1,
    Salt = 2,
    PublicKey = 3,
    Proof = 4,
    EncryptedData = 5,
    PairingState = 6,
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
#[derive(Display, Debug, Copy, Clone, PartialEq, Eq, FromRepr)]
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

pub trait TypedCode {
    const CODE: u8;
}

pub trait Tlv8 {
    const TAG: TagCode;
    type Param;
    type Value;

    fn length(value: &Self::Value) -> usize;
}

pub trait Tlv8Pack {
    type Param;
    type Value;
}

impl<T: Tlv8> Tlv8Pack for T {
    type Param = T::Param;
    type Value = T::Value;
}

macro_rules! impl_tlv8_pack {
    ( $( $name:ident )+ ) => {
        impl<$($name: Tlv8),+> Tlv8Pack for ( $($name,)+ ) {
            type Param = ( $($name::Param,)+ );
            type Value = ( $($name::Value,)+ );
        }
    };
}

impl_tlv8_pack!(T1 T2);
impl_tlv8_pack!(T1 T2 T3);
impl_tlv8_pack!(T1 T2 T3 T4);
impl_tlv8_pack!(T1 T2 T3 T4 T5);
impl_tlv8_pack!(T1 T2 T3 T4 T5 T6);

macro_rules! impl_tlv8_for_code {
    (
        $mod_name:ident,
        $name:ident,
        $trait_name:ident { $($variant:ident = $code:expr),+ $(,)? }
    ) => {
        pub struct $name<T: $trait_name>(std::marker::PhantomData<T>);

        pub trait $trait_name: TypedCode {
        }

        impl<T: $trait_name> Tlv8 for $name<T> {
            const TAG: TagCode = TagCode::$name;
            type Param = T;
            type Value = ();

            fn length(_: &Self::Value) -> usize {
                1
            }
        }

        pub mod $mod_name {
            $(
                pub struct $variant;
                impl super::TypedCode for $variant {
                    const CODE: u8 = $code;
                }
                impl super::$trait_name for $variant {}
            )+
        }
    };
}

impl_tlv8_for_code!(method, Method, MethodCode {
    PairSetup = 0,
    PairSetupAuth = 1,
    PairVerify = 2,
    AddPairing = 3,
    RemovePairing = 4,
    ListPairings = 5,
});
impl_tlv8_for_code!(state, PairingState, StateCode {
    M1 = 1,
    M2 = 2,
    M3 = 3,
    M4 = 4,
    M5 = 5,
    M6 = 6,
});

macro_rules! impl_tlv8 {
    ($name: ident, $value: ty, $length: expr) => {
        pub struct $name;
        impl Tlv8 for $name {
            const TAG: TagCode = TagCode::$name;
            type Param = ();
            type Value = $value;

            fn length(value: &Self::Value) -> usize {
                ($length)(value)
            }
        }
    };
}

impl_tlv8!(Salt, Vec<u8>, |v: &[u8]| v.len());
impl_tlv8!(PublicKey, Vec<u8>, |v: &[u8]| v.len());
impl_tlv8!(Proof, Vec<u8>, |v: &[u8]| v.len());

impl Tlv8 for ErrorCode {
    const TAG: TagCode = TagCode::Error;
    type Param = ();
    type Value = Self;

    fn length(_: &Self::Value) -> usize {
        mem::size_of::<Self>()
    }
}

impl Tlv8 for PairingFlags {
    const TAG: TagCode = TagCode::Flags;
    type Param = ();
    type Value = Self;

    fn length(_: &Self::Value) -> usize {
        mem::size_of::<u32>()
    }
}

bitflags! {
    #[derive(Debug, Default, PartialEq)]
    pub struct PairingFlags: u32 {
        const TRANSIENT = 1 << 4;
        const SPLIT_SETUP = 1 << 24;
    }
}

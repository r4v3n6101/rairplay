use std::iter;

use thiserror::Error;

use super::{
    super::dto::{ErrorCode, PairingFlags, TagCode, Tlv8, TypedCode},
    Tlv8Decode, Tlv8Encode, Tlv8Rejection,
};

#[derive(Debug, Error)]
pub enum DecodingError {
    #[error("invalid length, expected at least {0}")]
    InvalidLength(usize),
    #[error("unexpected value: {unexpected}, expected: {expected}")]
    UnexpectedValue { unexpected: u8, expected: u8 },
    #[error("invalid bitmask")]
    InvalidBitmask,
}

impl<T> Tlv8Decode for T
where
    T: Tlv8,
    T::Value: Decode<T::Param>,
{
    fn from_iter<'a, I>(iter: I) -> Result<Self::Value, Tlv8Rejection>
    where
        I: IntoIterator<Item = (TagCode, &'a [u8])> + Clone,
    {
        let mut iter = iter
            .into_iter()
            .filter(|(tag, _)| *tag == T::TAG)
            .flat_map(|(_, data)| data)
            .copied()
            .peekable();
        if iter.peek().is_none() {
            return Err(Tlv8Rejection::MissingTag(T::TAG));
        }
        Ok(T::Value::decode(iter)?)
    }
}

impl<T> Tlv8Encode for T
where
    T: Tlv8,
    T::Value: Encode<T::Param>,
{
    fn bytes_iter(value: Self::Value) -> impl Iterator<Item = u8> {
        // Tag + 1 byte len + 255 bytes max of data
        const CHUNK_LEN: usize = 0xFF + 2;

        let len = T::length(&value);
        let mut data = value.encode();

        let mut i = 0;
        iter::from_fn(move || {
            let res = match (i / CHUNK_LEN, i % CHUNK_LEN) {
                (chunk_no, 0) if chunk_no * 0xFF == len => None,
                (_, 0) => Some(T::TAG as u8),
                (chunk_no, 1) => Some((len - chunk_no * 0xFF).min(0xFF) as u8),
                _ => data.next(),
            };

            i += 1;
            res
        })
    }
}

macro_rules! impl_tlv8_for_tuples {
    ( $( $name:ident )+ ) => {
        impl<$($name: Tlv8),+> Tlv8Encode for ( $($name,)+ )
        where
            $($name::Value: Encode<$name::Param>,)+
        {
            fn bytes_iter(value: Self::Value) -> impl Iterator<Item = u8> {
                #[allow(non_snake_case)]
                let ($($name,)+) = value;
                std::iter::empty()
                    $(.chain($name::bytes_iter($name)))+
            }
        }

        impl<$($name: Tlv8),+> Tlv8Decode for ( $($name,)+ )
        where
            $($name::Value: Decode<$name::Param>,)+
        {
            fn from_iter<'a, I>(iter: I) -> Result<Self::Value, Tlv8Rejection>
            where
                I: IntoIterator<Item = (TagCode, &'a [u8])> + Clone,
            {
                Ok((
                    $(
                        $name::from_iter(
                            iter.clone()
                        )?,
                    )+
                ))
            }
        }
    };
}

impl_tlv8_for_tuples!(T1 T2);
impl_tlv8_for_tuples!(T1 T2 T3);
impl_tlv8_for_tuples!(T1 T2 T3 T4);
impl_tlv8_for_tuples!(T1 T2 T3 T4 T5);
impl_tlv8_for_tuples!(T1 T2 T3 T4 T5 T6);

trait Encode<P> {
    fn encode(self) -> impl Iterator<Item = u8>;
}

trait Decode<P>: Sized {
    fn decode(iter: impl Iterator<Item = u8>) -> Result<Self, DecodingError>;
}

impl<P: TypedCode> Encode<P> for () {
    fn encode(self) -> impl Iterator<Item = u8> {
        iter::once(P::CODE)
    }
}

impl<P: TypedCode> Decode<P> for () {
    fn decode(mut iter: impl Iterator<Item = u8>) -> Result<Self, DecodingError> {
        let Some(value) = iter.next() else {
            return Err(DecodingError::InvalidLength(1));
        };

        if value != P::CODE {
            return Err(DecodingError::UnexpectedValue {
                unexpected: value,
                expected: P::CODE,
            });
        }

        Ok(())
    }
}

impl<T> Encode<()> for T
where
    T: IntoIterator<Item = u8>,
{
    fn encode(self) -> impl Iterator<Item = u8> {
        self.into_iter()
    }
}

impl<T> Decode<()> for T
where
    T: FromIterator<u8>,
{
    fn decode(iter: impl Iterator<Item = u8>) -> Result<Self, DecodingError> {
        Ok(iter.collect())
    }
}

impl Encode<()> for PairingFlags {
    fn encode(self) -> impl Iterator<Item = u8> {
        self.bits().to_be_bytes().into_iter()
    }
}

impl Decode<()> for PairingFlags {
    fn decode(mut iter: impl Iterator<Item = u8>) -> Result<Self, DecodingError> {
        let bytes = [
            iter.next().ok_or(DecodingError::InvalidLength(4))?,
            iter.next().ok_or(DecodingError::InvalidLength(3))?,
            iter.next().ok_or(DecodingError::InvalidLength(2))?,
            iter.next().ok_or(DecodingError::InvalidLength(1))?,
        ];

        PairingFlags::from_bits(u32::from_be_bytes(bytes)).ok_or(DecodingError::InvalidBitmask)
    }
}

impl Encode<()> for ErrorCode {
    fn encode(self) -> impl Iterator<Item = u8> {
        iter::once(self as u8)
    }
}

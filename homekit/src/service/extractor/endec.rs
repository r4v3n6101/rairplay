use std::iter;

use thiserror::Error;

use super::{
    super::dto::{ErrorCode, MethodCode, StateCode, TagCode, Tlv8, Tlv8Pack},
    Tlv8Rejection,
};

// TODO : merge into rejection actually
#[derive(Debug, Error)]
pub enum DecodingError {
    #[error("invalid length, expected at least {0}")]
    InvalidLength(usize),
    #[error("unexpected value: {0}")]
    UnexpectedValue(u8),
}

pub trait Tlv8Encode: Tlv8Pack {
    fn bytes_iter(value: Self::Value) -> impl Iterator<Item = u8>;
}

pub trait Tlv8Decode: Tlv8Pack {
    // TODO : error
    fn from_iter<'a, I>(iter: I) -> Result<Self::Value, Tlv8Rejection>
    where
        I: IntoIterator<Item = (TagCode, &'a [u8])> + Clone;
}

impl<T> Tlv8Decode for T
where
    T: Tlv8,
    T::Value: Decode,
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
    T::Value: Encode,
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
            $($name::Value: Encode,)+
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
            $($name::Value: Decode,)+
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
impl_tlv8_for_tuples!(T1 T2 T3 T4 T5 T6 T7);
impl_tlv8_for_tuples!(T1 T2 T3 T4 T5 T6 T7 T8);
impl_tlv8_for_tuples!(T1 T2 T3 T4 T5 T6 T7 T8 T9);
impl_tlv8_for_tuples!(T1 T2 T3 T4 T5 T6 T7 T8 T9 T10);

trait Encode {
    fn encode(self) -> impl Iterator<Item = u8>;
}

trait Decode: Sized {
    fn decode(iter: impl Iterator<Item = u8>) -> Result<Self, DecodingError>;
}

macro_rules! impl_endec_for_code {
    ($name: ident) => {
        impl Encode for $name {
            fn encode(self) -> impl Iterator<Item = u8> {
                iter::once(self as _)
            }
        }

        impl Decode for $name {
            fn decode(mut iter: impl Iterator<Item = u8>) -> Result<Self, DecodingError> {
                let Some(value) = iter.next() else {
                    return Err(DecodingError::InvalidLength(1));
                };

                Self::from_repr(value).ok_or(DecodingError::UnexpectedValue(value))
            }
        }
    };
}

impl_endec_for_code!(MethodCode);
impl_endec_for_code!(StateCode);
impl_endec_for_code!(ErrorCode);

impl<I> Encode for I
where
    I: IntoIterator<Item = u8>,
{
    fn encode(self) -> impl Iterator<Item = u8> {
        self.into_iter()
    }
}

impl<C> Decode for C
where
    C: FromIterator<u8>,
{
    fn decode(iter: impl Iterator<Item = u8>) -> Result<Self, DecodingError> {
        Ok(iter.collect())
    }
}

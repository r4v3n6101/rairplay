use std::fmt;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PinError {
    #[error("sequence of digits not allowed")]
    NotAllowed,
    #[error("only digits allowed, not number")]
    InvalidDigit,
}

// 8x u8 for easy alignments, don't really wanna do u32 math
#[derive(Debug, Copy, Clone)]
pub struct PinCode(pub [u8; 8]);

impl fmt::Display for PinCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [s, u, p, w, o, r, l, d] = self.0;

        write!(f, "{s}{u}{p}-{w}{o}-{r}{l}{d}")
    }
}

impl TryFrom<[u8; 8]> for PinCode {
    type Error = PinError;

    fn try_from(value: [u8; 8]) -> Result<Self, Self::Error> {
        const INVALID_CODES: &[[u8; 8]] = &[
            [0, 0, 0, 0, 0, 0, 0, 0],
            [1, 1, 1, 1, 1, 1, 1, 1],
            [2, 2, 2, 2, 2, 2, 2, 2],
            [3, 3, 3, 3, 3, 3, 3, 3],
            [4, 4, 4, 4, 4, 4, 4, 4],
            [5, 5, 5, 5, 5, 5, 5, 5],
            [6, 6, 6, 6, 6, 6, 6, 6],
            [7, 7, 7, 7, 7, 7, 7, 7],
            [8, 8, 8, 8, 8, 8, 8, 8],
            [9, 9, 9, 9, 9, 9, 9, 9],
            [1, 2, 3, 4, 5, 6, 7, 8],
            [8, 7, 6, 5, 4, 3, 2, 1],
        ];

        if INVALID_CODES.contains(&value) {
            return Err(PinError::NotAllowed);
        }

        if value.iter().any(|x| *x >= 10) {
            return Err(PinError::InvalidDigit);
        }

        Ok(Self(value))
    }
}

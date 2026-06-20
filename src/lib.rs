extern crate alloc;

pub mod timelock;

#[cfg(target_arch = "wasm32")]
mod wasm;

use alloc::string::String;

pub const TL_BYTES: usize = 256;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("expected {expected} bytes, got {got}")]
    BadLength { expected: usize, got: usize },
    #[error("modulus must be odd")]
    BadModulus,
    #[error("invalid hex")]
    BadHex(#[from] hex::FromHexError),
}

pub type Result<T> = core::result::Result<T, Error>;

/// A 256-byte big-endian value
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Wide([u8; TL_BYTES]);

impl Wide {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; TL_BYTES]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; TL_BYTES] {
        &self.0
    }

    #[must_use]
    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self> {
        let raw = hex::decode(s)?;
        if raw.len() > TL_BYTES {
            return Err(Error::BadLength { expected: TL_BYTES, got: raw.len() });
        }
        let mut bytes = [0u8; TL_BYTES];
        bytes[TL_BYTES - raw.len()..].copy_from_slice(&raw);
        Ok(Self(bytes))
    }
}

impl core::fmt::Debug for Wide {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Wide({}...)", hex::encode(&self.0[..8]))
    }
}

impl TryFrom<&[u8]> for Wide {
    type Error = Error;

    fn try_from(slice: &[u8]) -> Result<Self> {
        let bytes: [u8; TL_BYTES] = slice
            .try_into()
            .map_err(|_| Error::BadLength { expected: TL_BYTES, got: slice.len() })?;
        Ok(Self(bytes))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Difficulty(pub u64);
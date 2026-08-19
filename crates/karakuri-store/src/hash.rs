//! Content addresses.
//!
//! An artifact is identified by the SHA-256 of its `.kir` source and nothing
//! else. Notably `capacity` does not enter into it beyond appearing in the
//! source as a declared range: the same procedure at 65536 and at 524288 is one
//! artifact with one hash and one preview, because capacity is a performance
//! dial turned per Set rather than part of a procedure's identity.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A `sha256:` prefixed content address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn of(bytes: &[u8]) -> Hash {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(bytes);
        Hash(hasher.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The first `n` hex characters, for logs and file names.
    pub fn short(&self, n: usize) -> String {
        self.to_hex().chars().take(n).collect()
    }

    fn to_hex(self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sha256:{}", self.to_hex())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HashParseError {
    #[error("content address must start with `sha256:`")]
    MissingPrefix,
    #[error("content address must be 64 hex characters, found {0}")]
    BadLength(usize),
    #[error("content address contains a non-hex character")]
    NotHex,
}

impl FromStr for Hash {
    type Err = HashParseError;

    fn from_str(s: &str) -> Result<Hash, HashParseError> {
        let hex = s
            .strip_prefix("sha256:")
            .ok_or(HashParseError::MissingPrefix)?;
        if hex.len() != 64 {
            return Err(HashParseError::BadLength(hex.len()));
        }
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                .map_err(|_| HashParseError::NotHex)?;
        }
        Ok(Hash(out))
    }
}

impl Serialize for Hash {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Hash, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_round_trips() {
        let h = Hash::of(b"proc drift_shell {}");
        assert_eq!(h.to_string().parse::<Hash>().unwrap(), h);
    }

    #[test]
    fn hashing_is_content_addressed() {
        assert_eq!(Hash::of(b"same"), Hash::of(b"same"));
        assert_ne!(Hash::of(b"same"), Hash::of(b"different"));
    }

    #[test]
    fn rejects_malformed_addresses() {
        assert_eq!(
            "deadbeef".parse::<Hash>(),
            Err(HashParseError::MissingPrefix)
        );
        assert_eq!(
            "sha256:ab".parse::<Hash>(),
            Err(HashParseError::BadLength(2))
        );
        assert_eq!(
            "sha256:".to_string().parse::<Hash>(),
            Err(HashParseError::BadLength(0))
        );
    }
}

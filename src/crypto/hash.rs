use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }
}

impl From<&[u8; 32]> for Hash {
    fn from(bytes: &[u8; 32]) -> Self {
        Hash(*bytes)
    }
}

impl From<&[u8]> for Hash {
    fn from(bytes: &[u8]) -> Self {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(bytes);
        Hash(hash)
    }
}

impl Default for Hash {
    fn default() -> Self {
        Hash([0u8; 32])
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0.iter() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

pub trait Hashable {
    fn hash(&self) -> Hash;
}

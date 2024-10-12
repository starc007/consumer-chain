use crate::crypto::{Hash, Hashable, PublicKey};
use serde::{Deserialize, Serialize};
use std::hash::{Hash as StdHash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: PublicKey,
    pub to: PublicKey,
    pub amount: u64,
    pub nonce: u64,
    pub signature: Vec<u8>,
}

impl StdHash for Transaction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.from.hash(state);
        self.to.hash(state);
        self.amount.hash(state);
        self.nonce.hash(state);
    }
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from
            && self.to == other.to
            && self.amount == other.amount
            && self.nonce == other.nonce
    }
}

impl Eq for Transaction {}

impl Transaction {
    pub fn new(from: PublicKey, to: PublicKey, amount: u64, nonce: u64) -> Self {
        Transaction {
            from,
            to,
            amount,
            nonce,
            signature: Vec::new(),
        }
    }

    pub fn sign(&mut self, signature: Vec<u8>) {
        self.signature = signature;
    }

    pub fn verify(&self) -> bool {
        // Implement signature verification logic here
        // This would typically involve checking the signature against the transaction data and the 'from' public key
        true // Placeholder
    }
}

impl Hashable for Transaction {
    fn hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.from.as_bytes());
        hasher.update(self.to.as_bytes());
        hasher.update(&self.amount.to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());
        Hash::from(hasher.finalize().as_bytes())
    }
}

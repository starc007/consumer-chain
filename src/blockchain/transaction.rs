use crate::crypto::{Hash, Hashable, PublicKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: PublicKey,
    pub to: PublicKey,
    pub amount: u64,
    pub nonce: u64,
    pub signature: Vec<u8>,
}

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

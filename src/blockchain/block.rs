use crate::crypto::{Hash, Hashable, PublicKey};

use crate::blockchain::transaction::Transaction;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub previous_hash: Hash,
    pub merkle_root: Hash,
    pub timestamp: u64,
    pub height: u64,
    pub validator: PublicKey,
}

impl Block {
    pub fn new(
        previous_hash: Hash,
        transactions: Vec<Transaction>,
        height: u64,
        validator: PublicKey,
    ) -> Self {
        let header = BlockHeader {
            version: 1,
            previous_hash,
            merkle_root: Self::calculate_merkle_root(&transactions),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs(),
            height,
            validator,
        };

        Block {
            header,
            transactions,
        }
    }

    fn calculate_merkle_root(transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash::default();
        }

        let mut hashes: Vec<Hash> = transactions.iter().map(|tx| tx.hash()).collect();

        while hashes.len() > 1 {
            if hashes.len() % 2 != 0 {
                hashes.push(hashes.last().unwrap().clone());
            }

            let mut new_hashes = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut hasher = blake3::Hasher::new();
                hasher.update(chunk[0].as_bytes());
                hasher.update(chunk[1].as_bytes());
                new_hashes.push(Hash::from(hasher.finalize().as_bytes()));
            }

            hashes = new_hashes;
        }

        hashes[0]
    }
}

impl Hashable for Block {
    fn hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.header.version.to_le_bytes());
        hasher.update(self.header.previous_hash.as_bytes());
        hasher.update(self.header.merkle_root.as_bytes());
        hasher.update(&self.header.timestamp.to_le_bytes());
        hasher.update(&self.header.height.to_le_bytes());
        hasher.update(self.header.validator.as_bytes());
        Hash::from(hasher.finalize().as_bytes())
    }
}

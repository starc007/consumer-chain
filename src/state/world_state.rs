use crate::blockchain::block::Block;
use crate::blockchain::transaction::Transaction;
use crate::crypto::{Hash, Hashable, PublicKey};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct Account {
    pub balance: u64,
    pub nonce: u64,
}

pub struct WorldState {
    accounts: Arc<RwLock<HashMap<PublicKey, Account>>>,
    last_block_hash: Hash,
}

impl WorldState {
    pub fn new() -> Self {
        WorldState {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            last_block_hash: Hash::default(),
        }
    }

    pub fn apply_block(&mut self, block: &Block) -> Result<(), String> {
        if block.header.previous_hash != self.last_block_hash {
            return Err("Invalid previous block hash".to_string());
        }

        for tx in &block.transactions {
            self.apply_transaction(tx)?;
        }

        self.last_block_hash = block.hash();
        Ok(())
    }

    fn apply_transaction(&mut self, tx: &Transaction) -> Result<(), String> {
        let mut accounts = self.accounts.write().unwrap();

        let from_account = accounts.entry(tx.from.clone()).or_insert(Account {
            balance: 0,
            nonce: 0,
        });
        if from_account.nonce != tx.nonce {
            return Err("Invalid nonce".to_string());
        }
        if from_account.balance < tx.amount {
            return Err("Insufficient balance".to_string());
        }

        from_account.balance -= tx.amount;
        from_account.nonce += 1;

        let to_account = accounts.entry(tx.to.clone()).or_insert(Account {
            balance: 0,
            nonce: 0,
        });
        to_account.balance += tx.amount;

        Ok(())
    }

    pub fn get_account(&self, public_key: &PublicKey) -> Option<Account> {
        self.accounts.read().unwrap().get(public_key).cloned()
    }

    pub fn get_last_block_hash(&self) -> Hash {
        self.last_block_hash.clone()
    }
}

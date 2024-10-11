use crate::blockchain::block::Block;
use crate::crypto::{Hash, PublicKey};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
enum PbftState {
    PrePrepare,
    Prepare,
    Commit,
}

pub struct PBFT {
    validators: HashSet<PublicKey>,
    state: PbftState,
    current_block: Option<Block>,
    prepare_messages: HashMap<PublicKey, Hash>,
    commit_messages: HashMap<PublicKey, Hash>,
}

impl PBFT {
    pub fn new(validators: HashSet<PublicKey>) -> Self {
        PBFT {
            validators,
            state: PbftState::PrePrepare,
            current_block: None,
            prepare_messages: HashMap::new(),
            commit_messages: HashMap::new(),
        }
    }

    pub fn on_propose_block(&mut self, block: Block) -> bool {
        if self.state != PbftState::PrePrepare {
            return false;
        }

        self.current_block = Some(block);
        self.state = PbftState::Prepare;
        true
    }

    pub fn on_prepare_message(&mut self, validator: PublicKey, block_hash: Hash) -> bool {
        if self.state != PbftState::Prepare {
            return false;
        }

        if !self.validators.contains(&validator) {
            return false;
        }

        self.prepare_messages.insert(validator, block_hash);

        if self.prepare_messages.len() > 2 * self.validators.len() / 3 {
            self.state = PbftState::Commit;
            return true;
        }

        false
    }

    pub fn on_commit_message(&mut self, validator: PublicKey, block_hash: Hash) -> bool {
        if self.state != PbftState::Commit {
            return false;
        }

        if !self.validators.contains(&validator) {
            return false;
        }

        self.commit_messages.insert(validator, block_hash);

        if self.commit_messages.len() > 2 * self.validators.len() / 3 {
            // Block is finalized
            self.reset();
            return true;
        }

        false
    }

    fn reset(&mut self) {
        self.state = PbftState::PrePrepare;
        self.current_block = None;
        self.prepare_messages.clear();
        self.commit_messages.clear();
    }
}

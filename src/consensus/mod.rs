pub mod dpos;
pub mod pbft;

use self::dpos::DPoS;
use self::pbft::PBFT;
use crate::blockchain::block::Block;
use crate::crypto::PublicKey;
use std::collections::HashSet;

pub struct ConsensusManager {
    dpos: DPoS,
    pbft: PBFT,
}

impl ConsensusManager {
    pub fn new(validators: HashSet<PublicKey>) -> Self {
        ConsensusManager {
            dpos: DPoS::new(),
            pbft: PBFT::new(validators),
        }
    }

    pub fn add_validator(&mut self, public_key: PublicKey, stake: u64) {
        self.dpos.add_validator(public_key, stake);
    }

    pub fn remove_validator(&mut self, public_key: &PublicKey) {
        self.dpos.remove_validator(public_key);
    }

    pub fn update_stake(&mut self, public_key: &PublicKey, new_stake: u64) {
        self.dpos.update_stake(public_key, new_stake);
    }

    pub fn can_produce_block(&self) -> bool {
        self.dpos.can_produce_block()
    }

    pub fn get_next_validator(&mut self) -> PublicKey {
        self.dpos.get_next_validator()
    }

    pub fn on_block_produced(&mut self, block: Block) -> bool {
        if !self.dpos.is_valid_block_producer(&block) {
            return false;
        }

        self.dpos.on_block_produced();
        self.pbft.on_propose_block(block)
    }
    pub fn on_prepare_message(
        &mut self,
        validator: PublicKey,
        block_hash: crate::crypto::Hash,
    ) -> bool {
        self.pbft.on_prepare_message(validator, block_hash)
    }

    pub fn on_commit_message(
        &mut self,
        validator: PublicKey,
        block_hash: crate::crypto::Hash,
    ) -> bool {
        self.pbft.on_commit_message(validator, block_hash)
    }
}

// src/consensus/dpos.rs

use crate::blockchain::block::Block;
use crate::crypto::PublicKey;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const BLOCK_TIME: Duration = Duration::from_secs(3);
const VALIDATOR_COUNT: usize = 21;

pub struct DPoS {
    validators: HashMap<PublicKey, u64>,
    active_validators: Vec<PublicKey>,
    total_stake: u64,
    current_validator_index: usize,
    last_block_time: Instant,
}

impl DPoS {
    pub fn new() -> Self {
        DPoS {
            validators: HashMap::new(),
            active_validators: Vec::new(),
            total_stake: 0,
            current_validator_index: 0,
            last_block_time: Instant::now(),
        }
    }

    pub fn add_validator(&mut self, public_key: PublicKey, stake: u64) {
        self.validators.insert(public_key.clone(), stake);
        self.total_stake += stake;
        self.update_active_validators();
    }

    pub fn remove_validator(&mut self, public_key: &PublicKey) {
        if let Some(stake) = self.validators.remove(public_key) {
            self.total_stake -= stake;
            self.update_active_validators();
        }
    }

    pub fn update_stake(&mut self, public_key: &PublicKey, new_stake: u64) {
        if let Some(old_stake) = self.validators.get_mut(public_key) {
            self.total_stake = self.total_stake - *old_stake + new_stake;
            *old_stake = new_stake;
            self.update_active_validators();
        }
    }

    fn update_active_validators(&mut self) {
        let mut validators: Vec<_> = self.validators.iter().collect();
        validators.sort_by(|a, b| b.1.cmp(a.1));
        self.active_validators = validators
            .into_iter()
            .take(VALIDATOR_COUNT)
            .map(|(k, _)| k.clone())
            .collect();
    }

    pub fn get_next_validator(&mut self) -> PublicKey {
        let validator = self.active_validators[self.current_validator_index].clone();
        self.current_validator_index =
            (self.current_validator_index + 1) % self.active_validators.len();
        validator
    }

    pub fn is_valid_block_producer(&self, block: &Block) -> bool {
        let expected_validator = &self.active_validators[self.current_validator_index];
        &block.header.validator == expected_validator
    }

    pub fn can_produce_block(&self) -> bool {
        Instant::now().duration_since(self.last_block_time) >= BLOCK_TIME
    }

    pub fn on_block_produced(&mut self) {
        self.last_block_time = Instant::now();
        self.current_validator_index =
            (self.current_validator_index + 1) % self.active_validators.len();
    }
}

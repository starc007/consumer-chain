use crate::blockchain::Block;
use crate::blockchain::transaction::Transaction;
use crate::consensus::ConsensusManager;
use crate::crypto::{Hash, PublicKey,Hashable};
use crate::network::P2PNetwork;
use crate::state::WorldState;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::marker::PhantomData;
use tokio::sync::RwLock;

pub struct Blockchain {
    blocks: Arc<RwLock<HashMap<Hash, Block>>>,
    latest_block_hash: Arc<RwLock<Hash>>,
    world_state: Arc<RwLock<WorldState>>,
    consensus_manager: Arc<RwLock<ConsensusManager>>,
    network: Arc<RwLock<P2PNetwork>>, // Change this line
    _phantom: PhantomData<()>,
}

impl Blockchain {
    pub fn new(
        consensus_manager: ConsensusManager,
        network: P2PNetwork,
    ) -> Self {
        let genesis_validator = PublicKey::genesis();
        let genesis_block = Block::new(
            Hash::default(),
            vec![],
            0,
            genesis_validator
        );
        let genesis_hash = genesis_block.hash();

        let mut blocks = HashMap::new();
        blocks.insert(genesis_hash.clone(), genesis_block);

        Blockchain {
            blocks: Arc::new(RwLock::new(blocks)),
            latest_block_hash: Arc::new(RwLock::new(genesis_hash)),
            world_state: Arc::new(RwLock::new(WorldState::new())),
            consensus_manager: Arc::new(RwLock::new(consensus_manager)),
            network: Arc::new(RwLock::new(network)), // Wrap network in RwLock
            _phantom: PhantomData,
        }
    }

    pub async fn add_block(&self, block: Block) -> Result<(), Box<dyn Error>> {
        let mut blocks = self.blocks.write().await;
        let mut latest_block_hash = self.latest_block_hash.write().await;
        let mut world_state = self.world_state.write().await;
        let mut consensus_manager = self.consensus_manager.write().await;

        if !consensus_manager.on_block_produced(block.clone()) {
            return Err("Block rejected by consensus".into());
        }

        if block.header.previous_hash != *latest_block_hash {
            return Err("Invalid previous block hash".into());
        }

        world_state.apply_block(&block)?;

        let block_hash = block.hash();
        blocks.insert(block_hash.clone(), block);
        *latest_block_hash = block_hash;

        Ok(())
    }

    pub async fn add_transaction(&self, transaction: Transaction) -> Result<(), Box<dyn Error>> {
        // Validate transaction
        if !transaction.verify() {
            return Err("Invalid transaction signature".into());
        }

        // Add to mempool (not implemented in this example)
        // Broadcast transaction to network
        let mut network = self.network.write().await;
        network.handle_network_message(crate::network::p2p::NetworkMessage::NewTransaction(transaction.clone())).await;

        // Optionally, you might want to add the transaction to a local mempool here
        // self.add_to_mempool(transaction);

        Ok(())
    }

    pub async fn get_latest_block(&self) -> Block {
        let latest_block_hash = self.latest_block_hash.read().await;
        let blocks = self.blocks.read().await;
        blocks.get(&latest_block_hash).unwrap().clone()
    }

    pub async fn get_block_by_hash(&self, hash: &Hash) -> Option<Block> {
        let blocks = self.blocks.read().await;
        blocks.get(hash).cloned()
    }

    pub async fn get_account_balance(&self, public_key: &PublicKey) -> u64 {
        let world_state = self.world_state.read().await;
        world_state.get_account(public_key).map_or(0, |account| account.balance)
    }
}
use crate::blockchain::transaction::Transaction;
use crate::blockchain::Block;
use crate::consensus::ConsensusManager;
use crate::crypto::{Hash, Hashable, PublicKey};
use crate::network::P2PNetwork;
use crate::state::WorldState;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Blockchain {
    blocks: Arc<RwLock<HashMap<Hash, Block>>>,
    latest_block_hash: Arc<RwLock<Hash>>,
    world_state: Arc<RwLock<WorldState>>,
    consensus_manager: Arc<RwLock<ConsensusManager>>,
    network: Arc<RwLock<Option<Arc<RwLock<P2PNetwork>>>>>,
    mempool: Arc<RwLock<HashSet<Transaction>>>,
}

impl Blockchain {
    pub fn new(consensus_manager: ConsensusManager) -> Self {
        let genesis_validator = PublicKey::genesis();
        let genesis_block = Block::new(Hash::default(), vec![], 0, genesis_validator);
        let genesis_hash = genesis_block.hash();

        let mut blocks = HashMap::new();
        blocks.insert(genesis_hash.clone(), genesis_block);

        Blockchain {
            blocks: Arc::new(RwLock::new(blocks)),
            latest_block_hash: Arc::new(RwLock::new(genesis_hash)),
            world_state: Arc::new(RwLock::new(WorldState::new())),
            consensus_manager: Arc::new(RwLock::new(consensus_manager)),
            network: Arc::new(RwLock::new(None)),
            mempool: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn set_network(&mut self, network: Arc<RwLock<P2PNetwork>>) {
        let mut net = self.network.write().await;
        *net = Some(network);
    }

    pub async fn get_network(&self) -> Option<Arc<RwLock<P2PNetwork>>> {
        let net = self.network.read().await;
        net.as_ref().cloned()
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

        // Check if the sender has sufficient balance
        let sender_balance = self.get_account_balance(&transaction.from).await;
        if sender_balance < transaction.amount {
            return Err("Insufficient balance".into());
        }

        self.add_to_mempool(transaction.clone()).await?;

        // Broadcast transaction to network
        if let Some(network) = self.get_network().await {
            let mut network = network.write().await;
            network.broadcast_transaction(transaction).await?;
        } else {
            return Err("Network not initialized".into());
        }

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
        world_state
            .get_account(public_key)
            .map_or(0, |account| account.balance)
    }

    async fn add_to_mempool(&self, transaction: Transaction) -> Result<(), Box<dyn Error>> {
        let mut mempool = self.mempool.write().await;
        if mempool.insert(transaction.clone()) {
            Ok(())
        } else {
            Err("Transaction already in mempool".into())
        }
    }

    pub async fn mine_block(&mut self) -> Result<Block, Box<dyn Error>> {
        // Get pending transactions from mempool
        let transactions = self.get_transactions_from_mempool().await?;

        // Create a new block
        let previous_hash = self.get_latest_block_hash().await?;
        let height = self.get_chain_length().await?;
        let miner_address = self.get_miner_address(); // You need to implement this method

        let new_block = Block::new(previous_hash, transactions, height, miner_address);

        // Add the new block to the chain
        self.add_block(new_block.clone()).await?;

        Ok(new_block)
    }

    async fn get_transactions_from_mempool(&self) -> Result<Vec<Transaction>, Box<dyn Error>> {
        let mempool = self.mempool.read().await;
        Ok(mempool.iter().cloned().collect())
    }

    async fn get_latest_block_hash(&self) -> Result<Hash, Box<dyn Error>> {
        let latest_block_hash = self.latest_block_hash.read().await;
        Ok(latest_block_hash.clone())
    }

    async fn get_chain_length(&self) -> Result<u64, Box<dyn Error>> {
        let blocks = self.blocks.read().await;
        Ok(blocks.len() as u64)
    }
    fn get_miner_address(&self) -> PublicKey {
        // Implement this method to return the miner's public key
        // This could be a fixed value or derived from a configuration
        unimplemented!()
    }
}

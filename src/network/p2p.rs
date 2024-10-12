use crate::blockchain::block::Block;
use crate::blockchain::chain::Blockchain;
use crate::blockchain::transaction::Transaction;
use crate::crypto::{Hash, Hashable};
use futures::prelude::*;
use libp2p::core::either::EitherError;
use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, ProtocolsHandlerUpgrErr, SwarmBuilder, SwarmEvent,Swarm},
    tcp::TokioTcpConfig,
    NetworkBehaviour, PeerId, Transport,
};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::io;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use void::Void;
use libp2p::noise::{Keypair, NoiseConfig, X25519Spec};
use libp2p::core::upgrade::Version;
use libp2p::mplex::MplexConfig;

const BLOCK_TOPIC: &str = "blocks";
const TRANSACTION_TOPIC: &str = "transactions";
const BLOCK_REQUEST_TOPIC: &str = "block_requests";

#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
struct FluxBehaviour {
    floodsub: Floodsub,
    mdns: Mdns,
    #[behaviour(ignore)]
    response_sender: mpsc::UnboundedSender<NetworkMessage>,
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for FluxBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = event {
            if let Ok(msg) = serde_json::from_slice::<NetworkMessage>(&message.data) {
                if let Err(e) = self.response_sender.send(msg) {
                    error!("Error sending message through channel: {}", e);
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for FluxBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer_id, _multiaddr) in list {
                    self.floodsub.add_node_to_partial_view(peer_id);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer_id, _multiaddr) in list {
                    if !self.mdns.has_node(&peer_id) {
                        self.floodsub.remove_node_from_partial_view(&peer_id);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NetworkMessage {
    NewBlock(Block),
    NewTransaction(Transaction),
    BlockRequest(Hash),
    BlockResponse(Block),
}

pub struct P2PNetwork {
    swarm: Swarm<FluxBehaviour>,
    response_receiver: mpsc::UnboundedReceiver<NetworkMessage>,
    blockchain: Arc<RwLock<Blockchain>>,
    pending_block_requests: HashMap<Hash, mpsc::Sender<Block>>,
}

impl P2PNetwork {
    pub async fn new(blockchain: Arc<RwLock<Blockchain>>) -> Result<Self, Box<dyn StdError>> {
       let id_keys = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(id_keys.public());
        info!("Local peer id: {:?}", peer_id);

        let noise_keys = Keypair::<X25519Spec>::new()
            .into_authentic(&id_keys)
            .expect("Signing libp2p-noise static DH keypair failed.");

        let transport = TokioTcpConfig::new()
            .upgrade(Version::V1)
            .authenticate(NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(MplexConfig::new())
            .boxed();

        let (response_sender, response_receiver) = mpsc::unbounded_channel();

        let mut behaviour = FluxBehaviour {
            floodsub: Floodsub::new(peer_id),
            mdns: Mdns::new(Default::default()).await?,
            response_sender,
        };

        behaviour.floodsub.subscribe(Topic::new(BLOCK_TOPIC));
        behaviour.floodsub.subscribe(Topic::new(TRANSACTION_TOPIC));
        behaviour
            .floodsub
            .subscribe(Topic::new(BLOCK_REQUEST_TOPIC));

       let swarm = SwarmBuilder::new(transport, behaviour, peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

        Ok(P2PNetwork {
            swarm,
            response_receiver,
            blockchain,
            pending_block_requests: HashMap::new(),
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn StdError>> {
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                },
                message = self.response_receiver.recv() => {
                    if let Some(msg) = message {
                        self.handle_network_message(msg).await;
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn floodsub_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(message) => {
                if let Ok(msg) = serde_json::from_slice::<NetworkMessage>(&message.data) {
                    self.handle_network_message(msg).await;
                }
            }
            _ => {}
        }
    }

    async fn mdns_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                info!("Discovered {:?} nodes", list.len());
            }
            _ => {}
        }
    }

    async fn handle_swarm_event(
        &mut self,
        event: SwarmEvent<(), EitherError<ProtocolsHandlerUpgrErr<io::Error>, Void>>,
    ) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {:?}", address);
            }
            // SwarmEvent::Behaviour(behaviour_event) => match behaviour_event {
            //     FluxBehaviourEvent::Floodsub(event) => {
            //         self.floodsub_event(event).await;
            //     }
            //     FluxBehaviourEvent::Mdns(event) => {
            //         self.mdns_event(event).await;
            //     }
            // },
            _ => {}
        }
    }

    pub async fn handle_network_message(&mut self, message: NetworkMessage) {
        match message {
            NetworkMessage::NewBlock(block) => {
                info!("Received new block: {:?}", block.hash());
                let result = self.add_block_to_blockchain(block.clone()).await;
                match result {
                    Ok(_) => {
                        if let Err(e) = self.broadcast_block(block).await {
                            error!("Failed to broadcast block: {}", e);
                        }
                    }
                    Err(e) => error!("Failed to add block: {}", e),
                }
            }
            NetworkMessage::NewTransaction(transaction) => {
                info!("Received new transaction: {:?}", transaction.hash());
                let result = self
                    .add_transaction_to_blockchain(transaction.clone())
                    .await;
                match result {
                    Ok(_) => {
                        if let Err(e) = self.broadcast_transaction(transaction).await {
                            error!("Failed to broadcast transaction: {}", e);
                        }
                    }
                    Err(e) => error!("Failed to add transaction: {}", e),
                }
            }
            NetworkMessage::BlockRequest(hash) => {
                info!("Received block request for hash: {:?}", hash);
                if let Some(block) = self.get_block_from_blockchain(&hash).await {
                    if let Err(e) = self.send_block_response(block).await {
                        error!("Failed to send block response: {}", e);
                    }
                }
            }
            NetworkMessage::BlockResponse(block) => {
                info!("Received block response: {:?}", block.hash());
                if let Some(sender) = self.pending_block_requests.remove(&block.hash()) {
                    if let Err(e) = sender.send(block).await {
                        error!("Failed to send block to requester: {}", e);
                    }
                }
            }
        }
    }

    async fn add_block_to_blockchain(&self, block: Block) -> Result<(), Box<dyn StdError>> {
        let blockchain = self.blockchain.write().await;
        blockchain.add_block(block).await?;
        Ok(())
    }

    async fn add_transaction_to_blockchain(
        &self,
        transaction: Transaction,
    ) -> Result<(), Box<dyn StdError>> {
        let blockchain = self.blockchain.write().await;
        blockchain.add_transaction(transaction).await?;
        Ok(())
    }

    async fn get_block_from_blockchain(&self, hash: &Hash) -> Option<Block> {
        let blockchain = self.blockchain.read().await;
        blockchain.get_block_by_hash(hash).await
    }

    async fn broadcast_block(&mut self, block: Block) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = serde_json::to_vec(&NetworkMessage::NewBlock(block))?;
        self.swarm
            .behaviour_mut()
            .floodsub
            .publish(Topic::new(BLOCK_TOPIC), bytes);
        Ok(())
    }

    pub async fn broadcast_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = serde_json::to_vec(&NetworkMessage::NewTransaction(transaction))?;
        self.swarm
            .behaviour_mut()
            .floodsub
            .publish(Topic::new(TRANSACTION_TOPIC), bytes);
        Ok(())
    }

    async fn send_block_response(
        &mut self,
        block: Block,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = serde_json::to_vec(&NetworkMessage::BlockResponse(block))?;
        self.swarm
            .behaviour_mut()
            .floodsub
            .publish(Topic::new(BLOCK_REQUEST_TOPIC), bytes);
        Ok(())
    }

    pub async fn request_block(&mut self, hash: Hash) -> Result<Option<Block>, Box<dyn StdError>> {
        let (sender, mut receiver) = mpsc::channel(1);
        self.pending_block_requests.insert(hash.clone(), sender);

        let message = NetworkMessage::BlockRequest(hash);
        let bytes = serde_json::to_vec(&message)?;
        self.swarm
            .behaviour_mut()
            .floodsub
            .publish(Topic::new(BLOCK_REQUEST_TOPIC), bytes);

        // Wait for the response with a timeout
        match tokio::time::timeout(std::time::Duration::from_secs(30), receiver.recv()).await {
            Ok(Some(block)) => Ok(Some(block)),
            Ok(None) => Ok(None),
            Err(_) => Err("Block request timed out".into()),
        }
    }
}

#[derive(Debug)]
enum FluxBehaviourEvent {
    Floodsub(FloodsubEvent),
    Mdns(MdnsEvent),
}

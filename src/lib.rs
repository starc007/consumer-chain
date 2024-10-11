pub mod blockchain;
pub mod consensus;
pub mod crypto;
pub mod network;
pub mod state;

// Re-export main types for convenience
pub use blockchain::Blockchain;
pub use consensus::ConsensusManager;
pub use crypto::{verify_signature, KeyPair, PublicKey};
pub use network::P2PNetwork;
pub use state::WorldState;

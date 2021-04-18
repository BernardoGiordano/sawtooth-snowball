use sawtooth_sdk::consensus::engine::{Block, BlockId, PeerId, PeerInfo};
use sawtooth_sdk::consensus::service::Service;

use crate::config::{PhaseQueenConfig};
use crate::state::{PhaseQueenState};

/// Contains the core logic of the PhaseQueen node
pub struct PhaseQueenNode {
    /// Used for interactions with the validator
    pub service: Box<dyn Service>,
}

impl PhaseQueenNode {
    /// Construct a new PhaseQueen node
    ///
    /// If the node is the primary on start-up, it initializes a new block on the chain
    pub fn new(
        config: &PhaseQueenConfig,
        chain_head: Block,
        connected_peers: Vec<PeerInfo>,
        service: Box<dyn Service>,
        state: &mut PhaseQueenState,
    ) -> Self {
        let mut n = PhaseQueenNode {
            service,
        };

        state.chain_head = chain_head.block_id.clone();

        n.service.initialize_block(None).unwrap_or_else(|err| {
            error!("Couldn't initialize block on startup due to error: {}", err)
        });
        n
    }

    /// Check to see if the idle timeout has expired
    pub fn check_idle_timeout_expired(&mut self, state: &mut PhaseQueenState) -> bool {
        state.idle_timeout.check_expired()
    }

    /// Start the idle timeout
    pub fn start_idle_timeout(&self, state: &mut PhaseQueenState) {
        state.idle_timeout.start();
    }

    // ---------- Methods for handling Updates from the Validator ----------

    pub fn on_block_new(&mut self, block: Block, state: &mut PhaseQueenState) -> bool {
        info!("Got BlockNew: {}", hex::encode(&block.block_id));

        true
    }

    pub fn on_block_valid(&mut self, block_id: BlockId, state: &mut PhaseQueenState) -> bool {
        info!("Got BlockValid: {}", hex::encode(&block_id));

        true
    }

    pub fn on_block_invalid(&mut self, block_id: BlockId) -> bool {
        info!("Got BlockInvalid: {}", hex::encode(&block_id));

        true
    }

    pub fn on_block_commit(&mut self, block_id: BlockId, state: &mut PhaseQueenState) -> bool {
        info!("Got BlockCommit: {}", hex::encode(&block_id));

        true
    }

    pub fn on_peer_connected(&mut self, peer_id: PeerId, state: &mut PhaseQueenState) -> bool {
        info!("Got PeerConnected: {:?}", hex::encode(&peer_id));

        true
    }

}
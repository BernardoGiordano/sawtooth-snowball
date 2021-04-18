use sawtooth_sdk::consensus::engine::{Block, PeerInfo};
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

}
use sawtooth_sdk::consensus::engine::{Block, BlockId, PeerId, PeerInfo};
use sawtooth_sdk::consensus::service::Service;

use crate::config::{PolyShardConfig};
use crate::state::{PolyShardState};

use rand;

/// Contains the core logic of the PolyShard node
pub struct PolyShardNode {
    /// Used for interactions with the validator
    pub service: Box<dyn Service>,
    pub rng: rand::ThreadRng,
}

impl PolyShardNode {
    /// Construct a new PolyShard node
    pub fn new(
        config: &PolyShardConfig,
        chain_head: Block,
        connected_peers: Vec<PeerInfo>,
        service: Box<dyn Service>,
        state: &mut PolyShardState,
    ) -> Self {
        let mut n = PolyShardNode {
            service: service,
            rng: rand::thread_rng(),
        };

        state.chain_head = chain_head.block_id.clone();

        n
    }

    /// Check to see if the idle timeout has expired
    pub fn check_idle_timeout_expired(&mut self, state: &mut PolyShardState) -> bool {
        state.idle_timeout.check_expired()
    }

    /// Start the idle timeout
    pub fn start_idle_timeout(&self, state: &mut PolyShardState) {
        state.idle_timeout.start();
    }

    pub fn cancel_block(&mut self) {
        debug!("Canceling block");
        match self.service.cancel_block() {
            Ok(_) => {}
            Err(err) => {
                panic!("Failed to cancel block: {:?}", err);
            }
        };
    }

    /// At a regular interval, try to finalize a block when the primary is ready
    pub fn try_publish(&mut self, state: &mut PolyShardState) -> () {

    }

    fn broadcast_value(&mut self, message: &str, v: u8, state: &mut PolyShardState) {

    }

    // ---------- Methods for handling Updates from the Validator ----------

    pub fn on_block_new(&mut self, block: Block, state: &mut PolyShardState) -> bool {
        info!(
            "{}: Got BlockNew: {} / {}",
            state,
            block.block_num,
            hex::encode(&block.block_id)
        );
        trace!("Block details: {:?}", block);
        
        true
    }

    pub fn on_block_valid(&mut self, block_id: BlockId, state: &mut PolyShardState) -> bool {
        info!("Got BlockValid: {}", hex::encode(&block_id));

        true
    }

    pub fn on_block_invalid(&mut self, block_id: BlockId) -> bool {
        info!("Got BlockInvalid: {}", hex::encode(&block_id));

        true
    }

    pub fn on_block_commit(&mut self, block_id: BlockId, state: &mut PolyShardState) -> bool {
        info!("Got BlockCommit: {}", hex::encode(&block_id));

        true
    }

    pub fn on_peer_connected(&mut self, peer_id: PeerId, state: &mut PolyShardState) -> bool {
        info!("Got PeerConnected: {:?}", hex::encode(&peer_id));

        true
    }

    pub fn on_peer_message(&mut self, message: &str, state: &mut PolyShardState) -> bool {
        info!("Got PeerMessage with message={}", message);

        true
    }

    // ---------- Methods for handling state changes ----------

    pub fn handle_block_new(&mut self, block_id: BlockId, state: &mut PolyShardState) {
        
    }

}
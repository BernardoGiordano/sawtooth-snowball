use sawtooth_sdk::consensus::engine::{Block, BlockId, PeerId, PeerInfo};
use sawtooth_sdk::consensus::service::Service;

use crate::config::{SnowballConfig};
use crate::state::{SnowballState};

use rand;

/// Contains the core logic of the Snowball node
pub struct SnowballNode {
    /// Used for interactions with the validator
    pub service: Box<dyn Service>,
    pub rng: rand::ThreadRng,
}

impl SnowballNode {
    /// Construct a new Snowball node
    pub fn new(
        config: &SnowballConfig,
        chain_head: Block,
        connected_peers: Vec<PeerInfo>,
        service: Box<dyn Service>,
        state: &mut SnowballState,
    ) -> Self {
        let mut n = SnowballNode {
            service: service,
            rng: rand::thread_rng(),
        };

        state.chain_head = chain_head.block_id.clone();

        n
    }

    /// Check to see if the idle timeout has expired
    pub fn check_idle_timeout_expired(&mut self, state: &mut SnowballState) -> bool {
        state.idle_timeout.check_expired()
    }

    /// Start the idle timeout
    pub fn start_idle_timeout(&self, state: &mut SnowballState) {
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
    pub fn try_publish(&mut self, state: &mut SnowballState) -> () {

    }

    fn broadcast_value(&mut self, message: &str, v: u8, state: &mut SnowballState) {

    }

    // ---------- Methods for handling Updates from the Validator ----------

    pub fn on_block_new(&mut self, block: Block, state: &mut SnowballState) -> bool {
        info!(
            "{}: Got BlockNew: {} / {}",
            state,
            block.block_num,
            hex::encode(&block.block_id)
        );
        trace!("Block details: {:?}", block);
        
        true
    }

    pub fn on_block_valid(&mut self, block_id: BlockId, state: &mut SnowballState) -> bool {
        info!("Got BlockValid: {}", hex::encode(&block_id));

        true
    }

    pub fn on_block_invalid(&mut self, block_id: BlockId) -> bool {
        info!("Got BlockInvalid: {}", hex::encode(&block_id));

        true
    }

    pub fn on_block_commit(&mut self, block_id: BlockId, state: &mut SnowballState) -> bool {
        info!("Got BlockCommit: {}", hex::encode(&block_id));

        true
    }

    pub fn on_peer_connected(&mut self, peer_id: PeerId, state: &mut SnowballState) -> bool {
        info!("Got PeerConnected: {:?}", hex::encode(&peer_id));

        true
    }

    pub fn on_peer_message(&mut self, message: &str, state: &mut SnowballState) -> bool {
        info!("Got PeerMessage with message={}", message);

        true
    }

    // ---------- Methods for handling state changes ----------

    pub fn handle_block_new(&mut self, block_id: BlockId, state: &mut SnowballState) {
        
    }

}
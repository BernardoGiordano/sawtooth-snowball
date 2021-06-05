use sawtooth_sdk::consensus::engine::{Block, BlockId, PeerId, PeerInfo};
use sawtooth_sdk::consensus::service::Service;

use crate::config::{SnowballConfig};
use crate::state::{SnowballState, SnowballPhase, SnowballDecisionState};

use std::collections::HashSet;

use rand;
use rand::distributions::{Distribution, Uniform};

/// Contains the core logic of the Snowball node
pub struct SnowballNode {
    /// Used for interactions with the validator
    pub service: Box<dyn Service>,
    pub rng: rand::rngs::ThreadRng,
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

        // TODO: GENERALIZE: ONLY FIRST NODE CAN PROPOSE
        if state.order == 0 {
            n.service.initialize_block(None).unwrap_or_else(|err| {
                error!("Couldn't initialize block on startup due to error: {}", err)
            });
        }

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
        if state.phase != SnowballPhase::Idle {
            return;
        }

        // TODO: GENERALIZE: ONLY FIRST NODE CAN PROPOSE
        if state.order != 0 {
            return;
        }

        info!("{}: Attempting to summarize block", state);

        let summary = self.service
                .summarize_block()
                .expect("Failed to summarize block");

        match self.service.finalize_block(create_consensus(&summary)) {
            Ok(block_id) => {
                info!("{}: Publishing block {}", state, hex::encode(&block_id));
            }
            Err(err) => { error!("Could not finalize block: {}", err); }
        }
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

        // algorithm starts on block new message
        let my_decision = SnowballDecisionState::OK;
        state.current_color = my_decision.clone();
        state.last_color = my_decision.clone();
        state.confidence_counter = 0;
        state.decision_array = [0, 0];

        let sample = self.select_node_sample(state);

        // TODO LOGICA DA FARE

        state.switch_phase(SnowballPhase::Listening);
        
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

    // ---------- Helper methods ----------

    pub fn select_node_sample(&mut self, state: &mut SnowballState) -> HashSet<u64> {
        let step = Uniform::new(0, state.member_ids.len());
        let mut set = HashSet::<u64>::with_capacity(state.k as usize);

        while set.len() < state.k as usize {
            let choices: Vec<_> = step.sample_iter(&mut self.rng).take(1).collect();
            for choice in choices {
                if choice != state.order as usize {
                    set.insert(choice as u64);
                }
            }
        }
        info!("Set for node {:?}: {:?}", state.order, set);
        set
    }

}

fn create_consensus(summary: &[u8]) -> Vec<u8> {
    let mut consensus: Vec<u8> = Vec::from(&b"Snowball"[..]);
    consensus.extend_from_slice(summary);
    consensus
}
use sawtooth_sdk::consensus::engine::{Block, BlockId, PeerId, PeerInfo};
use sawtooth_sdk::consensus::service::Service;

use crate::config::{SnowballConfig};
use crate::state::{SnowballState, SnowballPhase, SnowballDecisionState};

use std::collections::HashSet;

use rand;
use rand::distributions::{Distribution, Uniform};

use safe_crypto::Nonce;

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

    fn send_peer_notification(&mut self, peer_id: &PeerId, message: &str) {
        self.send_peer_message(peer_id, message, 0);
    }

    fn send_peer_message(&mut self, peer_id: &PeerId, message: &str, v: u8) {
        debug!("Sending {} message to {:?}", message, peer_id);
        let nonce = Nonce::new().into_bytes();
        let mut payload = Vec::new();
        payload.append(&mut vec![v]);
        payload.append(&mut nonce.to_vec());

        self.service
            .send_to(&peer_id, message, payload)
            .expect("Failed to send message");
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

        // Only future blocks should be considered since committed blocks are final
        if block.block_num < state.seq_num {
            self.service
                .fail_block(block.block_id.clone())
                .unwrap_or_else(|err| error!("Couldn't fail block due to error: {:?}", err));
            warn!(
                "Received block {:?} / {:?} that is older than the current sequence number: {:?}",
                block.block_num,
                hex::encode(&block.block_id),
                state.seq_num,
            );
            return true;
        }

        self.service
            .check_blocks(vec![block.block_id.clone()])
            .expect("Failed to check block");

        self.handle_block_new(block.block_id, state);
        
        true
    }

    pub fn prepare_and_forward_peer_requests(&mut self, state: &mut SnowballState) {
        info!("Preparing new peer notifications.");
        state.response_buffer = [0, 0];
        let sample = self.select_node_sample(state);
        for index in sample {
            let peer_id = &state.member_ids[index];
            self.send_peer_notification(&peer_id, "request");
        }
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

    pub fn on_peer_message(&mut self, message: &str, sender_id: &PeerId, value: u8, state: &mut SnowballState) -> bool {
        info!("Got PeerMessage with message={}", message);

        match message {
            "request" => {
                if state.phase == SnowballPhase::Idle || state.current_color == SnowballDecisionState::Undecided {
                    return false;
                }
                let current_value: u8 = if state.current_color == SnowballDecisionState::OK { 1 } else { 0 };
                self.send_peer_message(sender_id, "response", current_value)
            }
            "response" => {
                if state.phase != SnowballPhase::Listening {
                    return false;
                }
                // TODO: CONTROLLARE SE IL MESSAGGIO AVVIENE DA UN PROCESSO PER
                // IL QUALE MI ASPETTAVO DI RICEVERLO
                if value != 0 && value != 1 {
                    return false;
                }
                state.response_buffer[value as usize] += 1;
                if state.response_buffer[0] + state.response_buffer[1] == state.k {
                    info!("Process {} received all the messages for this round: {:?}", state.order, state.response_buffer);
                    self.on_values_ready(state);
                }
            }
            _ => { }
        }

        true
    }

    pub fn usize_to_decision_state(&mut self, i: usize) -> SnowballDecisionState {
        if i == 0 {
            return SnowballDecisionState::KO;
        }
        else if i == 1 {
            return SnowballDecisionState::OK;
        }
        panic!("Invalid value: {}", i);
    }

    pub fn decision_state_to_u8(&mut self, decision: SnowballDecisionState) -> u8 {
        match decision {
            SnowballDecisionState::OK => 1,
            SnowballDecisionState::KO => 0,
            SnowballDecisionState::Undecided => { panic!("Invalid conversion: {}", decision); }
        }
    }

    pub fn on_values_ready(&mut self, state: &mut SnowballState) {
        info!("Processing on values ready for process {}", state.order);
        let mut majority = false;
        for i in 0..=1 {
            let col_i = self.usize_to_decision_state(i);
            debug!("Response buffer={:?} for {}, alfa={}", state.response_buffer, i, state.alfa);
            if state.response_buffer[i] >= state.alfa {
                majority = true;
                state.decision_array[i] += 1;
                if state.decision_array[i] > state.decision_array[state.current_color as usize] {
                    state.current_color = col_i;
                }
                if col_i != state.last_color {
                    state.last_color = col_i;
                    state.confidence_counter = 1
                }
                else {
                    state.confidence_counter += 1
                }
                if state.confidence_counter >= state.beta {
                    state.switch_phase(SnowballPhase::Finishing);
                    self.handle_decision(state)
                }
                else {
                    self.prepare_and_forward_peer_requests(state);
                }
            }
        }
        if !majority {
            state.confidence_counter = 0;
        }
    }

    // ---------- Methods for handling state changes ----------

    pub fn handle_block_new(&mut self, block_id: BlockId, state: &mut SnowballState) {
        state.decision_block = block_id;
        
        // algorithm starts on block new message
        let my_decision = SnowballDecisionState::OK;
        state.current_color = my_decision.clone();
        state.last_color = my_decision.clone();
        state.confidence_counter = 0;
        state.decision_array = [0, 0];

        self.prepare_and_forward_peer_requests(state);

        state.switch_phase(SnowballPhase::Listening);
    }

    pub fn handle_decision(&mut self, state: &mut SnowballState) {
        info!("Process {} deciding {} for block seq {}", state.order, state.current_color, state.seq_num);
        
        if state.current_color == SnowballDecisionState::OK {
            self.service
                .commit_block(state.decision_block.clone())
                .expect("Failed to commit block");

            state.seq_num += 1;
            state.chain_head = state.decision_block.clone();
        }
        else {
            self.service
                .fail_block(state.decision_block.clone())
                .expect("Failed to fail block");
        }

        // TODO: GENERALIZE: ONLY FIRST NODE CAN PROPOSE
        if state.order == 0 {
            self.service.initialize_block(None).unwrap_or_else(|err| {
                error!("Couldn't initialize block due to error: {}", err)
            });
        }

        state.switch_phase(SnowballPhase::Idle);
    }

    // ---------- Helper methods ----------

    pub fn select_node_sample(&mut self, state: &mut SnowballState) -> HashSet<usize> {
        let step = Uniform::new(0, state.member_ids.len());
        let mut set = HashSet::<usize>::with_capacity(state.k as usize);

        while set.len() < state.k as usize {
            let choices: Vec<_> = step.sample_iter(&mut self.rng).take(1).collect();
            for choice in choices {
                if choice != state.order as usize {
                    set.insert(choice);
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
use sawtooth_sdk::consensus::engine::{Block, BlockId, PeerId, PeerInfo};
use sawtooth_sdk::consensus::service::Service;

use crate::config::{SnowballConfig};
use crate::state::{SnowballState, SnowballPhase, SnowballDecisionState};
use crate::message::{SnowballMessage};

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
    // pub fn check_idle_timeout_expired(&mut self, state: &mut SnowballState) -> bool {
    //     state.idle_timeout.check_expired()
    // }

    /// Start the idle timeout
    // pub fn start_idle_timeout(&self, state: &mut SnowballState) {
    //     state.idle_timeout.start();
    // }

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

    fn send_peer_notification(&mut self, peer_id: &PeerId, message: &str, seq_num: u64) {
        self.send_peer_message(peer_id, message, 0, seq_num);
    }

    fn send_peer_message(&mut self, peer_id: &PeerId, message: &str, v: u8, seq_num: u64) {
        debug!("Sending {} message to {:?}", message, peer_id);
        let nonce = Nonce::new().into_bytes();
        let mut payload = SnowballMessage::new();
        payload.vote = v;
        payload.nonce = nonce.to_vec();
        payload.seq_num = seq_num;
        payload.message_type = String::from(message);

        self.service
            .send_to(&peer_id, message, serde_json::to_string(&payload).unwrap().as_bytes().to_vec())
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

    pub fn prepare_and_forward_peer_requests(&mut self, sample: HashSet<usize>, state: &mut SnowballState) {
        debug!("Preparing new peer notifications.");
        state.response_buffer = [0, 0];
        for index in sample {
            let peer_id = &state.member_ids[index];
            self.send_peer_notification(&peer_id, "request", state.seq_num);
            state.response_sample_ids.insert(peer_id.clone());
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

    pub fn on_peer_message(&mut self, message: &str, sender_id: &PeerId, payload: SnowballMessage, state: &mut SnowballState) -> bool {
        debug!("Got PeerMessage with message {}", message);

        if state.seq_num != payload.seq_num {
            warn!("Process {} received message for seq_num {} when it was on seq_num {}", state.order, payload.seq_num, state.seq_num);
        }

        match message {
            "request" => {
                if payload.seq_num > state.seq_num {
                    self.send_peer_notification(sender_id, "unavailable", payload.seq_num);
                    return false;
                }

                let seq_value = state.decision_map.get(&payload.seq_num);
                if seq_value == None {
                    error!("Process {} unable to find seq_num in map for seq_num {}. Doing nothing.", state.order, payload.seq_num);
                    return false;
                }

                let current_value: u8 = if *seq_value.unwrap() == SnowballDecisionState::OK { 1 } else { 0 };
                self.send_peer_message(sender_id, "response", current_value, state.seq_num);
            }
            "response" => {
                if state.phase != SnowballPhase::Listening {
                    warn!("Process {} received a response message when it was not listening. Current state: {}", state.order, state);
                    return false;
                }
                if !state.response_sample_ids.contains(sender_id) {
                    warn!("Process {} received unwaited message from {:?}", state.order, sender_id);
                    return false;
                }

                // a message arrived from a node I was waiting for a response, I
                // remove it from the waiting response set
                state.response_sample_ids.remove(sender_id);

                if payload.vote != 0 && payload.vote != 1 {
                    error!("Process {} received invalid vote ({}) from node {:?}", state.order, payload.vote, sender_id);
                    return false;
                }
                state.response_buffer[payload.vote as usize] += 1;
                if state.response_buffer[0] + state.response_buffer[1] == state.k {
                    info!("Process {} received all the messages for this round: {:?}", state.order, state.response_buffer);
                    self.on_values_ready(state);
                }
            }
            "unavailable" => {
                if state.phase != SnowballPhase::Listening {
                    warn!("Process {} received a `unexpected` message when it was not listening. Current state: {}", state.order, state);
                    return false;
                }

                if !state.response_sample_ids.contains(sender_id) {
                    warn!("Process {} received unwaited message from {:?}", state.order, sender_id);
                    return false;
                }

                // a message arrived from a node I was waiting for a response, I
                // remove it from the waiting response set
                state.response_sample_ids.remove(sender_id);

                // I find another node to send a request to, which is not in my
                // current waiting response set
                let mut peer_id = Vec::new();
                let missing_responses_len = state.response_sample_ids.len();
                while state.response_sample_ids.len() < missing_responses_len + 1 {
                    let extra_node_set = self.select_node_sample(state, 1);
                    for extra_node_index in extra_node_set {
                        peer_id = state.member_ids[extra_node_index].clone();
                        state.response_sample_ids.insert(peer_id.clone());
                    }
                }
                
                info!("Sending additional peer notifications to {:?}.", peer_id);
                self.send_peer_notification(&peer_id, "request", state.seq_num);
            }
            _ => { }
        }

        true
    }

    pub fn usize_to_decision_state(&mut self, i: usize) -> SnowballDecisionState {
        match i {
            0 => SnowballDecisionState::KO,
            1 => SnowballDecisionState::OK,
            _ => { panic!("Invalid value: {}", i); }
        }       
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
                let current_color = state.decision_map.get(&state.seq_num).unwrap();
                if state.decision_array[i] > state.decision_array[self.decision_state_to_u8(*current_color) as usize] {
                    state.decision_map.insert(state.seq_num, col_i);
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
                    let sample = self.select_node_sample(state, state.k as usize);
                    self.prepare_and_forward_peer_requests(sample, state);
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
        state.seq_num += 1;
        
        // algorithm starts on block new message
        let my_decision = SnowballDecisionState::OK;

        state.decision_map.insert(state.seq_num, my_decision.clone());
        state.last_color = my_decision.clone();
        state.confidence_counter = 0;
        state.decision_array = [0, 0];

        let sample = self.select_node_sample(state, state.k as usize);
        self.prepare_and_forward_peer_requests(sample, state);

        state.switch_phase(SnowballPhase::Listening);
    }

    pub fn handle_decision(&mut self, state: &mut SnowballState) {
        let decision = state.decision_map.get(&state.seq_num).unwrap();
        info!("Process {} deciding {} for block seq {}", state.order, decision, state.seq_num);
        
        if *decision == SnowballDecisionState::OK {
            self.service
                .commit_block(state.decision_block.clone())
                .expect("Failed to commit block");
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

    pub fn select_node_sample(&mut self, state: &mut SnowballState, amount: usize) -> HashSet<usize> {
        let step = Uniform::new(0, state.member_ids.len());
        let mut set = HashSet::<usize>::with_capacity(amount);

        while set.len() < amount as usize {
            let choices: Vec<_> = step.sample_iter(&mut self.rng).take(1).collect();
            for choice in choices {
                if choice != state.order as usize {
                    set.insert(choice);
                }
            }
        }
        debug!("Set for node {:?}: {:?}", state.order, set);
        set
    }

}

fn create_consensus(summary: &[u8]) -> Vec<u8> {
    let mut consensus: Vec<u8> = Vec::from(&b"Snowball"[..]);
    consensus.extend_from_slice(summary);
    consensus
}
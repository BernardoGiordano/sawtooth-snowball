use std::fmt;
use std::time::Duration;
use std::collections::{HashMap, HashSet};

use sawtooth_sdk::consensus::engine::{BlockId, PeerId};

use crate::timing::Timeout;
use crate::config::SnowballConfig;

/// Phases of the Snowball algorithm
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
pub enum SnowballPhase {
    Idle,
    Listening,
    Finishing,
}

/// Decision states of the Snowball algorithm
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize, Copy)]
pub enum SnowballDecisionState {
    OK,
    KO,
    Undecided
}

impl fmt::Display for SnowballPhase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SnowballPhase::Idle => "Idle",
                SnowballPhase::Listening => "Listening",
                SnowballPhase::Finishing => "Finishing",
            },
        )
    }
}

impl fmt::Display for SnowballDecisionState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SnowballDecisionState::OK => "OK",
                SnowballDecisionState::KO => "KO",
                SnowballDecisionState::Undecided => "Undecided",
            },
        )
    }
}

impl fmt::Display for SnowballState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(process {}, {}, seq {}, chain head: {:?})",
            self.order, self.phase, self.seq_num, self.chain_head
        )
    }
}

/// Information about the Snowball algorithm's state
#[derive(Debug, Serialize, Deserialize)]
pub struct SnowballState {
    /// This node's ID
    pub id: PeerId,

    /// This node order number in the member array
    pub order: u64,

    /// The node's current sequence number
    pub seq_num: u64,

    // Alfa parameter
    pub alfa: u64,

    // Beta parameter
    pub beta: u64,

    // Sample size
    pub k: u64,

    // Current color
    pub decision_map: HashMap<u64, SnowballDecisionState>,

    // Last color
    pub last_color: SnowballDecisionState,

    // Confidence counter
    pub confidence_counter: u64,

    // Response buffer
    pub response_buffer: [u64; 2],

    // Decision array
    pub decision_array: [u64; 2],

    // Set containing ids from peers we're waiting response
    pub response_sample_ids: HashSet<PeerId>,

    /// The block ID of the node's current chain head
    pub chain_head: BlockId,

    /// Block id to accept
    pub decision_block: BlockId,

    /// Current phase of the algorithm
    pub phase: SnowballPhase,

    /// List of members in the Snowball network, including this node
    pub member_ids: Vec<PeerId>,

    /// Timer used to make sure the primary publishes blocks in a timely manner
    pub idle_timeout: Timeout,

    /// The base time to use for retrying with exponential backoff
    pub exponential_retry_base: Duration,

    /// The maximum time for retrying with exponential backoff
    pub exponential_retry_max: Duration,
}

impl SnowballState {
    /// Construct the initial state for a Snowball node
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(id: PeerId, head_block_num: u64, config: &SnowballConfig) -> Self {

        let order: u64 = config.members.clone().iter().position(|x| x == &id).unwrap() as u64;

        let mut decision_map = HashMap::new();
        decision_map.insert(0, SnowballDecisionState::Undecided);

        SnowballState {
            id,
            order: order,
            seq_num: head_block_num + 1,
            alfa: config.alfa,
            beta: config.beta,
            k: config.k,
            decision_map: decision_map,
            last_color: SnowballDecisionState::Undecided,
            confidence_counter: 0,
            response_buffer: [0, 0],
            decision_array: [0, 0],
            response_sample_ids: HashSet::new(),
            chain_head: BlockId::new(),
            decision_block: BlockId::new(),
            phase: SnowballPhase::Idle,
            member_ids: config.members.clone(),
            idle_timeout: Timeout::new(config.idle_timeout),
            exponential_retry_base: config.exponential_retry_base,
            exponential_retry_max: config.exponential_retry_max,
        }
    }

    /// Switch to the desired phase if it is the next phase of the algorithm; if it is not the next
    /// phase, return an error
    pub fn switch_phase(&mut self, desired_phase: SnowballPhase) -> bool {
        info!("Trying to switch phase {} to {}", self.phase, desired_phase);

        let next_phase = match self.phase {
            SnowballPhase::Idle => SnowballPhase::Listening,
            SnowballPhase::Listening => SnowballPhase::Finishing,
            SnowballPhase::Finishing => SnowballPhase::Idle
        };

        self.phase = if next_phase == desired_phase {
            desired_phase 
        } else {
            error!("Node attempted to go to {} when in phase {}", desired_phase, self.phase);
            next_phase 
        };
        
        true
    }

}
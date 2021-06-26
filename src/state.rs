use std::fmt;
use std::iter::FromIterator;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
            "(process {}, {}, seq {}, chain head: {:?}, waiting_set: {:?}, response_buffer: {:?}, byzantine: {:?})",
            self.order, self.phase, self.seq_num, hex::encode(&self.chain_head), self.waiting_response_map,
            self.response_buffer, self.byzantine_test
        )
    }
}

/// Information about the Byzantine behaviour parameters for testing purposes
#[derive(Debug, Serialize, Deserialize)]
pub struct ByzantineParameters {
    pub enabled: bool,

    pub max_churn_timeout_millis: u64,

    pub churn_idx: HashSet<u64>,

    pub hang_idx: HashSet<u64>,

    pub max_sleep_delay_millis: u64,

    pub sleep_idx: HashSet<u64>,

    pub duplicate_idx: HashSet<u64>,

    pub spurious_idx: HashSet<u64>,

    pub wrong_decision_idx: HashSet<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Measurements {
    pub convergenza: HashMap<BlockId, u128>,

    pub n_messaggi_inviati: u64,
}

impl Measurements {
    pub fn new() -> Self {
        Measurements {
            convergenza: HashMap::new(),
            n_messaggi_inviati: 0
        }
    }
}

impl ByzantineParameters {
    pub fn new(config: &SnowballConfig) -> Self {
        ByzantineParameters {
            enabled: config.byzantine_enabled,
            max_churn_timeout_millis: config.byzantine_max_churn_timeout_millis,
            churn_idx: FromIterator::from_iter(config.byzantine_churn_idx.clone()),
            hang_idx: FromIterator::from_iter(config.byzantine_hang_idx.clone()),
            max_sleep_delay_millis: config.byzantine_max_sleep_delay_millis,
            sleep_idx: FromIterator::from_iter(config.byzantine_sleep_idx.clone()),
            duplicate_idx: FromIterator::from_iter(config.byzantine_duplicate_idx.clone()),
            spurious_idx: FromIterator::from_iter(config.byzantine_spurious_idx.clone()),
            wrong_decision_idx: FromIterator::from_iter(config.byzantine_wrong_decision_idx.clone()),
        }
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
    pub waiting_response_map: HashMap<PeerId, Timeout>,

    /// The block ID of the node's current chain head
    pub chain_head: BlockId,

    /// Block id to accept
    pub decision_block: BlockId,

    /// Current phase of the algorithm
    pub phase: SnowballPhase,

    /// List of members in the Snowball network, including this node
    pub member_ids: Vec<PeerId>,

    /// The base time to use for retrying with exponential backoff
    pub exponential_retry_base: Duration,

    /// The maximum time for retrying with exponential backoff
    pub exponential_retry_max: Duration,

    /// How long to wait before deciding a process is hung
    pub hang_timeout: Duration,

    // Byzantine parameters for testing purposes
    pub byzantine_test: ByzantineParameters,

    // Measurements variable to keep track of execution info
    pub measurements: Measurements,
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
            waiting_response_map: HashMap::new(),
            chain_head: BlockId::new(),
            decision_block: BlockId::new(),
            phase: SnowballPhase::Idle,
            member_ids: config.members.clone(),
            exponential_retry_base: config.exponential_retry_base,
            exponential_retry_max: config.exponential_retry_max,
            hang_timeout: config.hang_timeout,
            byzantine_test: ByzantineParameters::new(config),
            measurements: Measurements::new(),
        }
    }

    pub fn switch_phase(&mut self) {
        let next_phase = match self.phase {
            SnowballPhase::Idle => SnowballPhase::Listening,
            SnowballPhase::Listening => SnowballPhase::Finishing,
            SnowballPhase::Finishing => SnowballPhase::Idle
        };

        info!("Switching phase {} to {}", self.phase, next_phase);
        self.phase = next_phase;
    }

    pub fn get_order_index(&mut self, id: PeerId) -> u64 {
        self.member_ids.clone().iter().position(|x| x == &id).unwrap() as u64
    }

    pub fn add_to_waiting_set(&mut self, id: PeerId) {
        let mut timeout = Timeout::new(self.hang_timeout);
        timeout.start();
        self.waiting_response_map.insert(id, timeout);
    }

    pub fn set_message_sent(&mut self) {
        self.measurements.n_messaggi_inviati += 1;
    }

    pub fn set_block_new_timestamp(&mut self, block_id: BlockId) {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        self.measurements.convergenza.insert(block_id, current_time);
    }

    pub fn set_block_commit_timestamp(&mut self, block_id: BlockId) -> u128 {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let old_time = self.measurements.convergenza.get(&block_id).cloned().expect("Unable to find block_id in map!");
        self.measurements.convergenza.insert(block_id.clone(), current_time - old_time);
        info!("Elapsed {} ns for block {} and process {}", current_time - old_time, hex::encode(block_id), self.order);
        current_time - old_time
    }
}
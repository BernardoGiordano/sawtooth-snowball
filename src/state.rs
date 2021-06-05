use std::fmt;
use std::time::Duration;

use sawtooth_sdk::consensus::engine::{BlockId, PeerId};

use crate::timing::Timeout;
use crate::config::SnowballConfig;

/// Phases of the Snowball algorithm, in `Normal` mode
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
pub enum SnowballPhase {
    Idle,
    Finishing,
}

impl fmt::Display for SnowballPhase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SnowballPhase::Idle => "Idle",
                SnowballPhase::Finishing => "Finishing",
            },
        )
    }
}

impl fmt::Display for SnowballState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "({}, seq {})",
            self.phase, self.seq_num,
        )
    }
}

/// Information about the Snowball algorithm's state
#[derive(Debug, Serialize, Deserialize)]
pub struct SnowballState {
    /// This node's ID
    pub id: PeerId,

    /// This node order number for queening
    pub order: u64,

    /// The node's current sequence number
    pub seq_num: u64,

    // Alfa parameter
    pub alfa: u64,

    // Beta parameter
    pub beta: u64,

    // Sample size
    pub k: u64,

    /// The block ID of the node's current chain head
    pub chain_head: BlockId,

    /// Block id to accept
    pub decision_block: BlockId,

    /// Current phase of the algorithm
    pub phase: SnowballPhase,

    /// List of members in the Snowball network, including this node
    pub member_ids: Vec<PeerId>,

    /// Timer used to make sure the primary publishes blocks in a timely manner. 
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

        SnowballState {
            id,
            order: order,
            seq_num: head_block_num + 1,
            alfa: config.alfa,
            beta: config.beta,
            k: config.k,
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
        info!("TODO: Trying to switch phase {} to {}", self.phase, desired_phase);
        
        true
    }

}
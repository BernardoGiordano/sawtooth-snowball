use std::fmt;
use std::time::Duration;

use sawtooth_sdk::consensus::engine::{BlockId, PeerId};

use crate::timing::Timeout;
use crate::config::PolyShardConfig;

/// Phases of the PolyShard algorithm, in `Normal` mode
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
pub enum PolyShardPhase {
    Idle,
    Finishing,
}

impl fmt::Display for PolyShardPhase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PolyShardPhase::Idle => "Idle",
                PolyShardPhase::Finishing => "Finishing",
            },
        )
    }
}

impl fmt::Display for PolyShardState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "({}, seq {})",
            self.phase, self.seq_num,
        )
    }
}

/// Information about the PolyShard algorithm's state
#[derive(Debug, Serialize, Deserialize)]
pub struct PolyShardState {
    /// This node's ID
    pub id: PeerId,

    /// This node order number for queening
    pub order: u64,

    /// The node's current sequence number
    pub seq_num: u64,

    /// The block ID of the node's current chain head
    pub chain_head: BlockId,

    /// Block id to accept
    pub decision_block: BlockId,

    /// Current phase of the algorithm
    pub phase: PolyShardPhase,

    /// List of members in the PolyShard network, including this node
    pub member_ids: Vec<PeerId>,

    /// Timer used to make sure the primary publishes blocks in a timely manner. 
    pub idle_timeout: Timeout,

    /// The base time to use for retrying with exponential backoff
    pub exponential_retry_base: Duration,

    /// The maximum time for retrying with exponential backoff
    pub exponential_retry_max: Duration,
}

impl PolyShardState {
    /// Construct the initial state for a PolyShard node
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(id: PeerId, head_block_num: u64, config: &PolyShardConfig) -> Self {

        let order: u64 = config.members.clone().iter().position(|x| x == &id).unwrap() as u64;

        PolyShardState {
            id,
            order: order,
            seq_num: head_block_num + 1,
            chain_head: BlockId::new(),
            decision_block: BlockId::new(),
            phase: PolyShardPhase::Idle,
            member_ids: config.members.clone(),
            idle_timeout: Timeout::new(config.idle_timeout),
            exponential_retry_base: config.exponential_retry_base,
            exponential_retry_max: config.exponential_retry_max,
        }
    }

    /// Switch to the desired phase if it is the next phase of the algorithm; if it is not the next
    /// phase, return an error
    pub fn switch_phase(&mut self, desired_phase: PolyShardPhase) -> bool {
        info!("TODO: Trying to switch phase {} to {}", self.phase, desired_phase);
        
        true
    }

}
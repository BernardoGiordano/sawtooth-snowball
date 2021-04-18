use std::fmt;
use std::time::Duration;

use sawtooth_sdk::consensus::engine::{BlockId, PeerId};

use crate::timing::Timeout;
use crate::config::PhaseQueenConfig;

/// Phases of the PBFT algorithm, in `Normal` mode
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
pub enum PbftPhase {
    PrePreparing,
    Preparing,
    Committing,
    // Node is waiting for a BlockCommit (bool indicates if it's a catch-up commit)
    Finishing(bool),
}

impl fmt::Display for PbftPhase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PbftPhase::PrePreparing => "PrePreparing".into(),
                PbftPhase::Preparing => "Preparing".into(),
                PbftPhase::Committing => "Committing".into(),
                PbftPhase::Finishing(cu) => format!("Finishing {}", cu),
            },
        )
    }
}

impl fmt::Display for PhaseQueenState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let phase = {
            match self.phase {
                PbftPhase::PrePreparing => "PP".into(),
                PbftPhase::Preparing => "Pr".into(),
                PbftPhase::Committing => "Co".into(),
                PbftPhase::Finishing(cu) => format!("Fi({})", cu),
            }
        };
        write!(
            f,
            "({}, seq {})",
            phase, self.seq_num,
        )
    }
}

/// Information about the PBFT algorithm's state
#[derive(Debug, Serialize, Deserialize)]
pub struct PhaseQueenState {
    /// This node's ID
    pub id: PeerId,

    /// This node order number for queening
    pub order: u64,

    /// The node's current sequence number
    pub seq_num: u64,

    /// The block ID of the node's current chain head
    pub chain_head: BlockId,

    /// Current phase of the algorithm
    pub phase: PbftPhase,

    /// List of members in the PBFT network, including this node
    pub member_ids: Vec<PeerId>,

    /// The maximum number of faulty nodes in the network
    pub f: u64,

    /// Timer used to make sure the primary publishes blocks in a timely manner. 
    pub idle_timeout: Timeout,

    /// The base time to use for retrying with exponential backoff
    pub exponential_retry_base: Duration,

    /// The maximum time for retrying with exponential backoff
    pub exponential_retry_max: Duration,
}

impl PhaseQueenState {
    /// Construct the initial state for a PBFT node
    ///
    /// # Panics
    /// + If the network this node is on does not have enough nodes to be Byzantine fault tolernant
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(id: PeerId, head_block_num: u64, config: &PhaseQueenConfig) -> Self {

        // TODO PHASEQUEEN: questo va aggiornato con il numero minimo di nodi phasequeen
        // Maximum number of faulty nodes in this network. Panic if there are not enough nodes.
        
        let f = ((config.members.len() - 1) / 4) as u64;
        if f == 0 {
            panic!("This network does not contain enough nodes to be fault tolerant");
        }

        let order: u64 = config.members.clone().iter().position(|x| x == &id).unwrap() as u64;

        PhaseQueenState {
            id,
            order: order,
            seq_num: head_block_num + 1,
            chain_head: BlockId::new(),
            phase: PbftPhase::PrePreparing,
            f,
            member_ids: config.members.clone(),
            idle_timeout: Timeout::new(config.idle_timeout),
            exponential_retry_base: config.exponential_retry_base,
            exponential_retry_max: config.exponential_retry_max,
        }
    }

}
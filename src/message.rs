use std::fmt;
use crate::state::{ByzantineParameters};
use crate::config::{SnowballConfig};

#[derive(Serialize, Deserialize)]
pub struct SnowballMessage {
    pub message_type: String,
    pub seq_num: u64,
    pub vote: u8,
    pub nonce: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct LogMessage {
    pub seq_num: u64,
    pub n_messages: u64,
    pub n_members: u64,
    pub elapsed_time: u128,
    pub block_id: String,
    pub alfa: u64,
    pub beta: u64,
    pub k: u64,
    pub order: u64,
    pub decision: u8,
    pub hang_timeout: u64,
    pub byzantine: ByzantineParameters
}

impl LogMessage {
    pub fn new() -> LogMessage {
        let config = SnowballConfig::default();
        LogMessage {
            seq_num: 0,
            n_messages: 0,
            n_members: 0,
            elapsed_time: 0,
            block_id: String::new(),
            alfa: 0,
            beta: 0,
            k: 0,
            order: 0,
            decision: 0,
            hang_timeout: 0,
            byzantine: ByzantineParameters::new(&config)
        }
    }
}

impl SnowballMessage {
    pub fn new() -> SnowballMessage{ 
        SnowballMessage {
            message_type: String::from("undefined"), 
            seq_num: 0, 
            vote: 0, 
            nonce: Vec::new()
        }
    }
}

impl fmt::Display for SnowballMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(message_type={}, seq={}, vote={})",
            self.message_type, self.seq_num, self.vote
        )
    }
}

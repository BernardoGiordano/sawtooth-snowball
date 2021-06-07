use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::fmt;

#[derive(Serialize, Deserialize)]
pub struct SnowballMessage {
    pub message_type: String,
    pub seq_num: u64,
    pub vote: u8,
    pub nonce: Vec<u8>,
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
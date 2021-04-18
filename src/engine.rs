use std::fmt::{self, Write};
use std::str::FromStr;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time;

use crate::config::PhaseQueenConfig;

use sawtooth_sdk::consensus::{engine::*, service::Service};

pub struct DevmodeEngine {
    config: PhaseQueenConfig,
}

impl DevmodeEngine {
    pub fn new(config: PhaseQueenConfig) -> Self {
        DevmodeEngine { config }
    }
}

impl Engine for DevmodeEngine {
    #[allow(clippy::cognitive_complexity)]
    fn start(
        &mut self,
        updates: Receiver<Update>,
        mut service: Box<dyn Service>,
        startup_state: StartupState,
    ) -> Result<(), Error> {
        info!("Startup state received from validator: {:?}", startup_state);

        let StartupState {
            chain_head,
            peers,
            local_peer_info,
        } = startup_state;

        // Load on-chain settings
        self.config
            .load_settings(chain_head.block_id.clone(), &mut *service);

        info!("PhaseQueen config loaded: {:?}", self.config);

        loop {
            let incoming_message = updates.recv_timeout(time::Duration::from_millis(10));

            match incoming_message {
                Ok(update) => {
                    debug!("Received message: {}", message_type(&update));

                    match update {
                        Update::Shutdown => {
                            break;
                        }
                        _ => { }
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    error!("Disconnected from validator");
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }

        Ok(())
    }

    fn version(&self) -> String {
        "0.1".into()
    }

    fn name(&self) -> String {
        "Devmode".into()
    }

    fn additional_protocols(&self) -> Vec<(String, String)> {
        vec![]
    }
}

struct DisplayBlock<'b>(&'b Block);

impl<'b> fmt::Display for DisplayBlock<'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Block(")?;
        f.write_str(&self.0.block_num.to_string())?;
        write!(f, ", id: {}", to_hex(&self.0.block_id))?;
        write!(f, ", prev: {})", to_hex(&self.0.previous_id))
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut buf = String::new();
    for b in bytes {
        write!(&mut buf, "{:0x}", b).expect("Unable to write to string");
    }

    buf
}

fn message_type(update: &Update) -> &str {
    match *update {
        Update::PeerConnected(_) => "PeerConnected",
        Update::PeerDisconnected(_) => "PeerDisconnected",
        Update::PeerMessage(..) => "PeerMessage",
        Update::BlockNew(_) => "BlockNew",
        Update::BlockValid(_) => "BlockValid",
        Update::BlockInvalid(_) => "BlockInvalid",
        Update::BlockCommit(_) => "BlockCommit",
        Update::Shutdown => "Shutdown",
    }
}

pub enum DevmodeMessage {
    Ack,
    Published,
    Received,
}

impl FromStr for DevmodeMessage {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ack" => Ok(DevmodeMessage::Ack),
            "published" => Ok(DevmodeMessage::Published),
            "received" => Ok(DevmodeMessage::Received),
            _ => Err("Invalid message type"),
        }
    }
}

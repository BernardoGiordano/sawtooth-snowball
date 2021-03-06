use std::fmt::{self, Write};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};
use std::str;

use crate::timing;
use crate::storage::get_storage;
use crate::config::SnowballConfig;
use crate::state::SnowballState;
use crate::node::SnowballNode;
use crate::message::SnowballMessage;

use sawtooth_sdk::consensus::{engine::*, service::Service};

pub struct SnowballEngine {
    config: SnowballConfig,
}

impl SnowballEngine {
    pub fn new(config: SnowballConfig) -> Self {
        SnowballEngine { config }
    }
}

impl Engine for SnowballEngine {
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

        info!("Snowball config loaded: {:?}", self.config);

        let mut snowball_state = get_storage(&self.config.storage_location, || {
            SnowballState::new(
                local_peer_info.peer_id.clone(),
                chain_head.block_num,
                &self.config,
            )
        })
        .unwrap_or_else(|err| panic!("Failed to load state due to error: {}", err));

        info!("SnowballState state created: {}", **snowball_state.read());

        let mut block_publishing_ticker = timing::Ticker::new(self.config.block_publishing_delay);

        let mut node = SnowballNode::new(
            &self.config,
            chain_head,
            peers,
            service,
            &mut snowball_state.write(),
        );

        // TODO: debug, rimuovere poi
        let mut timestamp_log = Instant::now();

        // Byzantine fault test code for nodes randomly disconnecting from the
        // network
        let random_wait = node.random_value(self.config.byzantine_max_churn_timeout_millis as usize) as u64;
        info!("Process {} with random_wait {}", snowball_state.write().order, random_wait);
        let mut byzantine_churn_timeout = timing::Timeout::new(Duration::from_millis(random_wait as u64));
        byzantine_churn_timeout.start();

        loop {
            let incoming_message = updates.recv_timeout(Duration::from_millis(10));
            let state = &mut **snowball_state.write();

            // Simulate byzantine crash for testing purposes
            if state.byzantine_test.enabled && byzantine_churn_timeout.check_expired() && state.byzantine_test.churn_idx.contains(&state.order) {
                debug!("Byzantine process {} terminates unexpectedly after {:?}", state.order, byzantine_churn_timeout);
                break;
            }

            node.handle_queue(state);

            match handle_update(&mut node, incoming_message, state) {
                Ok(again) => {
                    if !again {
                        info!("Final state is: {:?}", state);
                        break;
                    }
                }
                Err(err) => error!("{}", err),
            }

            node.handle_unresponsive_peers(state);

            block_publishing_ticker.tick(|| node.try_publish(state));

            if Instant::now().duration_since(timestamp_log) > Duration::from_millis(4500) {
                info!("State log: {}", state);
                timestamp_log = Instant::now();
            }
        }

        info!("Process exited out of loop");

        Ok(())
    }

    fn version(&self) -> String {
        "0.1".into()
    }

    fn name(&self) -> String {
        "Snowball".into()
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

fn handle_update(
    node: &mut SnowballNode,
    incoming_message: Result<Update, RecvTimeoutError>,
    state: &mut SnowballState,
) -> Result<bool, Error> {
    match incoming_message {
        Ok(Update::BlockNew(block)) => node.on_block_new(block, state),
        Ok(Update::BlockValid(block_id)) => node.on_block_valid(block_id, state),
        Ok(Update::BlockInvalid(block_id)) => node.on_block_invalid(block_id),
        Ok(Update::BlockCommit(block_id)) => node.on_block_commit(block_id, state),
        Ok(Update::PeerMessage(message, sender_id)) => {
            let content_string = str::from_utf8(message.content.as_ref()).unwrap();
            let payload: SnowballMessage = serde_json::from_str(content_string).unwrap();
            // info!("Message content: {}", payload);
            node.on_peer_message(message.header.message_type.as_ref(), &sender_id, payload, state);
            return Ok(true);
        }
        Ok(Update::Shutdown) => {
            info!("Received shutdown; stopping Snowball.");
            return Ok(false);
        }
        Ok(Update::PeerConnected(info)) => {
            node.on_peer_connected(info.peer_id, state);
            return Ok(true);
        }
        Ok(Update::PeerDisconnected(id)) => {
            node.on_peer_disconnected(id, state);
            return Ok(true);
        }
        Err(RecvTimeoutError::Timeout) => { return Ok(true); },
        Err(RecvTimeoutError::Disconnected) => {
            error!("Disconnected from validator; stopping Snowball");
            return Ok(false);
        }
    };

    Ok(true)
}
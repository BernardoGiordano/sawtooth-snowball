//! Initial configuration for a Snowball node

use std::collections::HashMap;
use std::time::Duration;

use sawtooth_sdk::consensus::{
    engine::{BlockId, PeerId},
    service::Service,
};

use crate::timing::retry_until_ok;

/// Contains the initial configuration loaded from on-chain settings and local configuration. The
/// `members` list is required; all other settings are optional (defaults used in their absence)
#[derive(Debug)]
pub struct SnowballConfig {
    // Members of the Snowball network
    pub members: Vec<PeerId>,

    // Alfa: majority threshold
    pub alfa: u64,

    // Beta: confidence threshold
    pub beta: u64,

    // sample size
    pub k: u64,

    /// How long to wait in between trying to publish blocks
    pub block_publishing_delay: Duration,

    /// How long to wait before deciding a process is hung
    pub hang_timeout: Duration,

    /// How long to wait for an update to arrive from the validator
    pub update_recv_timeout: Duration,

    /// The base time to use for retrying with exponential backoff
    pub exponential_retry_base: Duration,

    /// The maximum time for retrying with exponential backoff
    pub exponential_retry_max: Duration,

    /// Where to store SnowballState ("memory" or "disk+/path/to/file")
    pub storage_location: String,

    pub byzantine_enabled: bool,

    pub byzantine_churn_idx: Vec<u64>,

    pub byzantine_max_churn_timeout_millis: u64,

    pub byzantine_hang_idx: Vec<u64>,

    pub byzantine_max_sleep_delay_millis: u64,

    pub byzantine_sleep_idx: Vec<u64>,

    pub byzantine_duplicate_idx: Vec<u64>,

    pub byzantine_spurious_idx: Vec<u64>,

    pub byzantine_wrong_decision_idx: Vec<u64>,
}

impl SnowballConfig {
    pub fn default() -> Self {
        SnowballConfig {
            members: Vec::new(),
            alfa: 0,
            beta: 0,
            k: 0,
            block_publishing_delay: Duration::from_millis(5000),
            hang_timeout: Duration::from_millis(3000),
            update_recv_timeout: Duration::from_millis(10),
            exponential_retry_base: Duration::from_millis(100),
            exponential_retry_max: Duration::from_millis(60000),
            storage_location: "memory".into(),
            byzantine_enabled: false,
            byzantine_max_churn_timeout_millis: 20000,
            byzantine_churn_idx: Vec::new(),
            byzantine_hang_idx: Vec::new(),
            byzantine_max_sleep_delay_millis: 6000,
            byzantine_sleep_idx: Vec::new(),
            byzantine_duplicate_idx: Vec::new(),
            byzantine_spurious_idx: Vec::new(),
            byzantine_wrong_decision_idx: Vec::new(),
        }
    }

    /// Load configuration from on-chain Sawtooth settings.
    ///
    /// Configuration loads the following settings:
    /// + `sawtooth.consensus.algorithm.members` (required)
    /// + `sawtooth.consensus.algorithm.alfa` (required)
    /// + `sawtooth.consensus.algorithm.beta` (required)
    /// + `sawtooth.consensus.algorithm.k` (required)
    /// + `sawtooth.consensus.algorithm.block_publishing_delay` (optional, default 10000 ms)
    /// TODO: document byzantine params
    ///
    /// # Panics
    /// + If the `sawtooth.consensus.algorithm.members` setting is not provided or is invalid
    /// + If the `sawtooth.consensus.algorithm.alfa` setting is not provided or is invalid
    /// + If the `sawtooth.consensus.algorithm.beta` setting is not provided or is invalid
    /// + If the `sawtooth.consensus.algorithm.k` setting is not provided or is invalid
    pub fn load_settings(&mut self, block_id: BlockId, service: &mut dyn Service) {
        debug!("Getting on-chain settings for config");
        let settings: HashMap<String, String> = retry_until_ok(
            self.exponential_retry_base,
            self.exponential_retry_max,
            || {
                service.get_settings(
                    block_id.clone(),
                    vec![
                        String::from("sawtooth.consensus.algorithm.members"),
                        String::from("sawtooth.consensus.algorithm.block_publishing_delay"),
                        String::from("sawtooth.consensus.algorithm.idle_timeout"),
                        String::from("sawtooth.consensus.algorithm.alfa"),
                        String::from("sawtooth.consensus.algorithm.beta"),
                        String::from("sawtooth.consensus.algorithm.k"),
                        String::from("sawtooth.consensus.algorithm.hang_timeout"),
                        String::from("sawtooth.byzantine.enabled"),
                        String::from("sawtooth.byzantine.parameter.max_churn_timeout"),
                        String::from("sawtooth.byzantine.parameter.churn_idx"),
                        String::from("sawtooth.byzantine.parameter.hang_idx"),
                        String::from("sawtooth.byzantine.parameter.max_sleep_delay"),
                        String::from("sawtooth.byzantine.parameter.sleep_idx"),
                        String::from("sawtooth.byzantine.parameter.duplicate_idx"),
                        String::from("sawtooth.byzantine.parameter.spurious_idx"),
                        String::from("sawtooth.byzantine.parameter.wrong_decision_idx"),
                    ],
                )
            },
        );

        // Get the on-chain list of Snowball members or panic if it is not provided; the network cannot
        // function without this setting, since there is no way of knowing which nodes are members.
        self.members = get_members_from_settings(&settings);

        self.alfa = settings
            .get("sawtooth.consensus.algorithm.alfa")
            .unwrap()
            .parse::<u64>()
            .expect("'sawtooth.consensus.algorithm.alfa' is empty; this setting must exist to use Snowball");

        self.beta = settings
            .get("sawtooth.consensus.algorithm.beta")
            .unwrap()
            .parse::<u64>()
            .expect("'sawtooth.consensus.algorithm.beta' is empty; this setting must exist to use Snowball");

        self.k = settings
            .get("sawtooth.consensus.algorithm.k")
            .unwrap()
            .parse::<u64>()
            .expect("'sawtooth.consensus.algorithm.k' is empty; this setting must exist to use Snowball");

        // Get durations
        merge_millis_setting_if_set(
            &settings,
            &mut self.block_publishing_delay,
            "sawtooth.consensus.algorithm.block_publishing_delay",
        );

        merge_millis_setting_if_set(
            &settings,
            &mut self.hang_timeout,
            "sawtooth.consensus.algorithm.hang_timeout",
        );

        // Configure byzantine parameters
        if let Some(setting) = settings.get("sawtooth.byzantine.enabled") {
            if let Ok(setting_value) = setting.parse() {
                self.byzantine_enabled = setting_value;
            }
        }

        if let Some(setting) = settings.get("sawtooth.byzantine.parameter.max_churn_timeout") {
            if let Ok(setting_value) = setting.parse() {
                self.byzantine_max_churn_timeout_millis = setting_value;
            }
        }

        if let Some(setting) = settings.get("sawtooth.byzantine.parameter.churn_idx") {
            if let Ok(setting_value) = serde_json::from_str(setting) {
                self.byzantine_churn_idx = setting_value;
            }
        }

        if let Some(setting) = settings.get("sawtooth.byzantine.parameter.hang_idx") {
            if let Ok(setting_value) = serde_json::from_str(setting) {
                self.byzantine_hang_idx = setting_value;
            }
        }

        if let Some(setting) = settings.get("sawtooth.byzantine.parameter.max_sleep_delay") {
            if let Ok(setting_value) = setting.parse() {
                self.byzantine_max_sleep_delay_millis = setting_value;
            }
        }

        if let Some(setting) = settings.get("sawtooth.byzantine.parameter.sleep_idx") {
            if let Ok(setting_value) = serde_json::from_str(setting) {
                self.byzantine_sleep_idx = setting_value;
            }
        }

        if let Some(setting) = settings.get("sawtooth.byzantine.parameter.duplicate_idx") {
            if let Ok(setting_value) = serde_json::from_str(setting) {
                self.byzantine_duplicate_idx = setting_value;
            }
        }

        if let Some(setting) = settings.get("sawtooth.byzantine.parameter.spurious_idx") {
            if let Ok(setting_value) = serde_json::from_str(setting) {
                self.byzantine_spurious_idx = setting_value;
            }
        }

        if let Some(setting) = settings.get("sawtooth.byzantine.parameter.wrong_decision_idx") {
            if let Ok(setting_value) = serde_json::from_str(setting) {
                self.byzantine_wrong_decision_idx = setting_value;
            }
        }

    }
}

fn merge_setting_if_set_and_map<U, F, T>(
    settings_map: &HashMap<String, String>,
    setting_field: &mut U,
    setting_key: &str,
    map: F,
) where
    F: Fn(T) -> U,
    T: ::std::str::FromStr,
{
    if let Some(setting) = settings_map.get(setting_key) {
        if let Ok(setting_value) = setting.parse() {
            *setting_field = map(setting_value);
        }
    }
}

fn merge_millis_setting_if_set(
    settings_map: &HashMap<String, String>,
    setting_field: &mut Duration,
    setting_key: &str,
) {
    merge_setting_if_set_and_map(
        settings_map,
        setting_field,
        setting_key,
        Duration::from_millis,
    )
}

/// Get the list of Snowball members as a Vec<PeerId> from settings
///
/// # Panics
/// + If the `sawtooth.consensus.algorithm.members` setting is unset or invalid
pub fn get_members_from_settings<S: std::hash::BuildHasher>(
    settings: &HashMap<String, String, S>,
) -> Vec<PeerId> {
    let members_setting_value = settings
        .get("sawtooth.consensus.algorithm.members")
        .expect("'sawtooth.consensus.algorithm.members' is empty; this setting must exist to use Snowball");

    let members: Vec<String> = serde_json::from_str(members_setting_value).unwrap_or_else(|err| {
        panic!(
            "Unable to parse value at 'sawtooth.consensus.algorithm.members' due to error: {:?}",
            err
        )
    });

    members
        .into_iter()
        .map(|s| {
            hex::decode(s).unwrap_or_else(|err| {
                panic!("Unable to parse PeerId from string due to error: {:?}", err)
            })
        })
        .collect()
}

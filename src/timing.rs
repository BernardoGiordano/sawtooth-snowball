//! Timing-related structures

use std::thread::sleep;
use std::time::{Duration, Instant};

/// Encapsulates calling a function every so often
pub struct Ticker {
    last: Instant,
    timeout: Duration,
}

impl Ticker {
    pub fn new(period: Duration) -> Self {
        Ticker {
            last: Instant::now(),
            timeout: period,
        }
    }

    // Do some work if the timeout has expired
    pub fn tick<T: FnMut()>(&mut self, mut callback: T) {
        let elapsed = Instant::now() - self.last;
        if elapsed >= self.timeout {
            callback();
            self.last = Instant::now();
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
enum TimeoutState {
    Active,
    Inactive,
    Expired,
}

/// A timer that expires after a given duration
/// Check back on this timer every so often to see if it's expired
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Timeout {
    state: TimeoutState,
    duration: Duration,
    #[serde(with = "serde_millis")]
    start: Instant,
}

impl Timeout {
    pub fn new(duration: Duration) -> Self {
        Timeout {
            state: TimeoutState::Inactive,
            duration,
            start: Instant::now(),
        }
    }

    /// Update the timer state, and check if the timer is expired
    pub fn check_expired(&mut self) -> bool {
        if self.state == TimeoutState::Active && Instant::now() - self.start > self.duration {
            self.state = TimeoutState::Expired;
        }
        match self.state {
            TimeoutState::Active | TimeoutState::Inactive => false,
            TimeoutState::Expired => true,
        }
    }

    pub fn start(&mut self) {
        self.state = TimeoutState::Active;
        self.start = Instant::now();
    }

    pub fn stop(&mut self) {
        self.state = TimeoutState::Inactive;
        self.start = Instant::now();
    }

    pub fn is_active(&self) -> bool {
        self.state == TimeoutState::Active
    }
}

/// With exponential backoff, repeatedly try the callback until the result is `Ok`
pub fn retry_until_ok<T, E, F: FnMut() -> Result<T, E>>(
    base: Duration,
    max: Duration,
    mut callback: F,
) -> T {
    let mut delay = base;
    loop {
        match callback() {
            Ok(res) => return res,
            Err(_) => {
                sleep(delay);
                // Only increase delay if it's less than the max
                if delay < max {
                    delay = delay
                        .checked_mul(2)
                        .unwrap_or_else(|| Duration::from_millis(std::u64::MAX));
                    // Make sure the max isn't exceeded
                    if delay > max {
                        delay = max;
                    }
                }
            }
        }
    }
}
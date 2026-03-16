// KLIK stdlib - Time module

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Get current time as milliseconds since Unix epoch
pub fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
}

/// Get current time as seconds since Unix epoch
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

/// Get current time as nanoseconds since Unix epoch
pub fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos()
}

/// A monotonic timer for measuring elapsed time
#[derive(Debug, Clone)]
pub struct Timer {
    start: Instant,
}

impl Timer {
    /// Create and start a new timer
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Elapsed time in milliseconds
    pub fn elapsed_millis(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    /// Elapsed time in microseconds
    pub fn elapsed_micros(&self) -> u128 {
        self.start.elapsed().as_micros()
    }

    /// Elapsed time in nanoseconds
    pub fn elapsed_nanos(&self) -> u128 {
        self.start.elapsed().as_nanos()
    }

    /// Elapsed time in seconds as f64
    pub fn elapsed_secs(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// Reset the timer
    pub fn reset(&mut self) {
        self.start = Instant::now();
    }
}

/// Sleep for the given number of milliseconds (blocking)
pub fn sleep_millis(millis: u64) {
    std::thread::sleep(Duration::from_millis(millis));
}

/// Sleep for the given number of seconds (blocking)
pub fn sleep_secs(secs: u64) {
    std::thread::sleep(Duration::from_secs(secs));
}

//! Login rate limiting: a small in-memory sliding window per key (client IP + email).
//! Enough to blunt password guessing on an internal server; a reverse proxy can add more.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    window: Duration,
    max: usize,
    hits: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new(max: usize, window: Duration) -> Self {
        Self {
            window,
            max,
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Records an attempt and returns `true` when the caller is still within the limit.
    pub fn allow(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut hits = self.hits.lock().unwrap();
        // Opportunistic cleanup so the map cannot grow without bound.
        if hits.len() > 10_000 {
            hits.retain(|_, v| v.iter().any(|t| now.duration_since(*t) < self.window));
        }
        let entry = hits.entry(key.to_owned()).or_default();
        entry.retain(|t| now.duration_since(*t) < self.window);
        if entry.len() >= self.max {
            return false;
        }
        entry.push(now);
        true
    }

    pub fn reset(&self, key: &str) {
        self.hits.lock().unwrap().remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_max_attempts() {
        let rl = RateLimiter::new(3, Duration::from_secs(60));
        assert!(rl.allow("a"));
        assert!(rl.allow("a"));
        assert!(rl.allow("a"));
        assert!(!rl.allow("a"));
        assert!(rl.allow("b"));
        rl.reset("a");
        assert!(rl.allow("a"));
    }
}

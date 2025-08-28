use std::collections::VecDeque;
use std::time::{Duration, Instant};
use std::collections::HashMap;

pub struct RateLimiter {
    hits: HashMap<String, VecDeque<Instant>>,
    per_minute: usize,
}
impl RateLimiter {
    pub fn new(per_minute: usize) -> Self {
        Self { hits: HashMap::new(), per_minute }
    }
    pub fn hit_ok(&mut self, user: &str) -> bool {
        let dq = self.hits.entry(user.to_string()).or_default();
        let now = Instant::now();
        while let Some(&t) = dq.front() {
            if now.duration_since(t) > Duration::from_secs(60) { dq.pop_front(); } else { break; }
        }
        if dq.len() >= self.per_minute { return false; }
        dq.push_back(now);
        true
    }
    pub fn sweep(&mut self) {
    let now = Instant::now();
    self.hits.retain(|_, dq| {
        while let Some(&t) = dq.front() {
            if now.duration_since(t) > Duration::from_secs(60) { dq.pop_front(); } else { break; }
        }
        !dq.is_empty()
    });
}
}
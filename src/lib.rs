/*!
# cuda-retry

Retry strategies and resilience patterns.

Networks fail. APIs timeout. Resources are temporarily unavailable.
This crate gives agents the resilience to handle failure gracefully
instead of crashing.

- Exponential backoff with jitter
- Circuit breaker (open/half-open/closed)
- Retry budget management
- Timeout tracking
- Success/failure statistics
*/

use serde::{Deserialize, Serialize};

/// Retry policy configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter: bool,
    pub timeout_ms: Option<u64>,
}

impl Default for RetryPolicy {
    fn default() -> Self { RetryPolicy { max_attempts: 3, base_delay_ms: 100, max_delay_ms: 30_000, backoff_multiplier: 2.0, jitter: true, timeout_ms: None } }
}

impl RetryPolicy {
    /// Create a policy with max attempts
    pub fn with_max(max: u32) -> Self { let mut p = RetryPolicy::default(); p.max_attempts = max; p }

    /// Calculate delay for attempt n (0-indexed)
    pub fn delay_for(&self, attempt: u32) -> u64 {
        if attempt == 0 { return 0; }
        let delay = self.base_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        let delay = delay.min(self.max_delay_ms as f64);
        let jitter = if self.jitter { (delay * 0.5) as u64 } else { 0 };
        (delay as u64).saturating_sub(jitter) // min side of jitter
    }
}

/// Circuit breaker state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState { Closed, Open, HalfOpen }

/// Circuit breaker
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitBreaker {
    pub state: CircuitState,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub open_since: Option<u64>,
    pub open_duration_ms: u64,
    pub total_opens: u32,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32) -> Self { CircuitBreaker { state: CircuitState::Closed, failure_threshold, success_threshold: 2, consecutive_failures: 0, consecutive_successes: 0, open_since: None, open_duration_ms: 30_000, total_opens: 0 } }

    /// Can we attempt the operation?
    pub fn allow(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(since) = self.open_since {
                    if now() - since >= self.open_duration_ms {
                        self.state = CircuitState::HalfOpen;
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a success
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        if self.state == CircuitState::HalfOpen {
            self.consecutive_successes += 1;
            if self.consecutive_successes >= self.success_threshold { self.state = CircuitState::Closed; }
        }
    }

    /// Record a failure
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        if self.state == CircuitState::HalfOpen { self.state = CircuitState::Open; self.open_since = Some(now()); }
        else if self.consecutive_failures >= self.failure_threshold {
            self.state = CircuitState::Open;
            self.open_since = Some(now());
            self.total_opens += 1;
        }
    }

    pub fn is_open(&self) -> bool { self.state == CircuitState::Open }
}

/// Attempt record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attempt {
    pub number: u32,
    pub success: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub timestamp: u64,
}

/// Retry operation tracker
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryTracker {
    pub policy: RetryPolicy,
    pub attempts: Vec<Attempt>,
    pub circuit_breaker: Option<CircuitBreaker>,
    pub total_retries: u64,
    pub total_failures: u64,
}

impl RetryTracker {
    pub fn new(policy: RetryPolicy) -> Self { RetryTracker { policy, attempts: vec![], circuit_breaker: None, total_retries: 0, total_failures: 0 } }

    pub fn with_circuit_breaker(mut self, threshold: u32) -> Self {
        self.circuit_breaker = Some(CircuitBreaker::new(threshold));
        self
    }

    /// Should we retry?
    pub fn should_retry(&self, attempt: u32) -> bool {
        if attempt >= self.policy.max_attempts { return false; }
        if let Some(ref cb) = self.circuit_breaker { if cb.is_open() { return false; } }
        true
    }

    /// Get delay before next attempt
    pub fn next_delay(&self, attempt: u32) -> u64 { self.policy.delay_for(attempt) }

    /// Record an attempt
    pub fn record(&mut self, attempt: u32, success: bool, duration_ms: u64, error: Option<&str>) {
        self.attempts.push(Attempt { number: attempt, success, duration_ms, error: error.map(|s| s.to_string()), timestamp: now() });
        if !success { self.total_failures += 1; }
        if attempt > 0 { self.total_retries += 1; }
        if let Some(ref mut cb) = self.circuit_breaker {
            if success { cb.record_success(); } else { cb.record_failure(); }
        }
    }

    /// Success rate
    pub fn success_rate(&self) -> f64 {
        if self.attempts.is_empty() { return 0.0; }
        self.attempts.iter().filter(|a| a.success).count() as f64 / self.attempts.len() as f64
    }

    /// Average duration of successful attempts
    pub fn avg_success_duration(&self) -> f64 {
        let successes: Vec<&Attempt> = self.attempts.iter().filter(|a| a.success).collect();
        if successes.is_empty() { return 0.0; }
        successes.iter().map(|a| a.duration_ms).sum::<u64>() as f64 / successes.len() as f64
    }

    /// Summary
    pub fn summary(&self) -> String {
        let cb_state = self.circuit_breaker.as_ref().map(|cb| format!("{:?}", cb.state)).unwrap_or_else(|| "none".into());
        format!("Retry: {}/{} attempts, rate={:.0}%, retries={}, circuit={}",
            self.attempts.iter().filter(|a| a.success).count(), self.attempts.len(),
            self.success_rate() * 100.0, self.total_retries, cb_state)
    }
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_delay() {
        let p = RetryPolicy::default();
        assert_eq!(p.delay_for(0), 0);
        assert_eq!(p.delay_for(1), 100); // base
    }

    #[test]
    fn test_exponential_backoff() {
        let p = RetryPolicy::default();
        let d1 = p.delay_for(1);
        let d2 = p.delay_for(2);
        assert!(d2 >= d1); // exponential
    }

    #[test]
    fn test_max_delay_cap() {
        let p = RetryPolicy { max_delay_ms: 500, ..RetryPolicy::default() };
        assert!(p.delay_for(10) <= 500);
    }

    #[test]
    fn test_circuit_breaker_closed() {
        let mut cb = CircuitBreaker::new(3);
        assert!(cb.allow());
        cb.record_success();
        assert_eq!(cb.state, CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens() {
        let mut cb = CircuitBreaker::new(3);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Open);
        assert!(!cb.allow());
    }

    #[test]
    fn test_circuit_breaker_half_open() {
        let mut cb = CircuitBreaker::new(2);
        cb.record_failure();
        cb.record_failure();
        assert!(cb.is_open());
        cb.open_since = Some(now() - 60_000); // simulate time passed
        assert!(cb.allow()); // transitions to half-open
        assert_eq!(cb.state, CircuitState::HalfOpen);
    }

    #[test]
    fn test_retry_tracker() {
        let mut tracker = RetryTracker::new(RetryPolicy::with_max(3));
        tracker.record(0, false, 100, Some("timeout"));
        tracker.record(1, true, 50, None);
        assert!(!tracker.should_retry(0)); // past max
        assert_eq!(tracker.total_retries, 1);
    }

    #[test]
    fn test_retry_with_circuit() {
        let mut tracker = RetryTracker::new(RetryPolicy::with_max(5)).with_circuit_breaker(2);
        tracker.record(0, false, 100, Some("err"));
        tracker.record(1, false, 100, Some("err"));
        assert!(!tracker.should_retry(2)); // circuit open
    }

    #[test]
    fn test_success_rate() {
        let mut tracker = RetryTracker::new(RetryPolicy::with_max(5));
        tracker.record(0, true, 10, None);
        tracker.record(1, false, 10, Some("err"));
        tracker.record(2, true, 10, None);
        tracker.record(3, true, 10, None);
        assert!((tracker.success_rate() - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_avg_duration() {
        let mut tracker = RetryTracker::new(RetryPolicy::with_max(3));
        tracker.record(0, true, 100, None);
        tracker.record(1, true, 200, None);
        assert!((tracker.avg_success_duration() - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_with_max_policy() {
        let p = RetryPolicy::with_max(5);
        assert_eq!(p.max_attempts, 5);
    }

    #[test]
    fn test_summary() {
        let tracker = RetryTracker::new(RetryPolicy::default());
        let s = tracker.summary();
        assert!(s.contains("circuit=none"));
    }
}

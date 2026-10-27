use std::hash::{BuildHasher, RandomState};
use std::time::Duration;

/// How many times a task may run and how long to wait between attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::none()
    }
}

impl RetryPolicy {
    /// Run once, never retry.
    pub const fn none() -> Self {
        Self {
            max_attempts: 1,
            base_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            jitter: false,
        }
    }

    /// Up to `max_attempts` runs in total (minimum 1), waiting
    /// `base_delay * 2^(n-1)` after the n-th failure, capped at 30 s by
    /// default, with jitter enabled.
    pub fn exponential(max_attempts: u32, base_delay: Duration) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            base_delay,
            max_delay: Duration::from_secs(30),
            jitter: true,
        }
    }

    /// Caps the delay between attempts.
    pub fn max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    /// Enables or disables jitter (each delay is scaled to 50–100%).
    pub fn jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Total attempts allowed.
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Delay after `failed_attempt` (1-based) failed.
    pub(crate) fn delay_after(&self, failed_attempt: u32, seed: u64) -> Duration {
        let factor = 2_u32.saturating_pow(failed_attempt.saturating_sub(1));
        let delay = self.base_delay.saturating_mul(factor).min(self.max_delay);
        if !self.jitter || delay.is_zero() {
            return delay;
        }
        let hash = RandomState::new().hash_one((seed, failed_attempt));
        let scale = 0.5 + (hash % 1_000) as f64 / 2_000.0;
        delay.mul_f64(scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_backoff_is_capped() {
        let policy = RetryPolicy::exponential(10, Duration::from_millis(100))
            .max_delay(Duration::from_millis(500))
            .jitter(false);
        let delays: Vec<_> = (1..=5)
            .map(|n| policy.delay_after(n, 0).as_millis())
            .collect();
        assert_eq!(delays, [100, 200, 400, 500, 500]);
    }

    #[test]
    fn jitter_stays_within_half_and_full_delay() {
        let policy = RetryPolicy::exponential(3, Duration::from_millis(1_000));
        for seed in 0..50 {
            let d = policy.delay_after(1, seed).as_millis();
            assert!((500..=1_000).contains(&d), "{d}");
        }
    }

    #[test]
    fn at_least_one_attempt() {
        assert_eq!(
            RetryPolicy::exponential(0, Duration::ZERO).max_attempts(),
            1
        );
        assert_eq!(RetryPolicy::none().max_attempts(), 1);
    }
}

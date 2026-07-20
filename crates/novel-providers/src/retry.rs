//! Deterministic retry and rate-limiting contracts.
//!
//! Providers implement exponential backoff with jitter. The core only sees
//! the final success or failure; retries are handled at the adapter level.
//! Rate limits are respected via `Retry-After` headers.

use std::time::Duration;

/// Retry configuration from a `ProviderProfile`.
#[derive(Debug, Clone, Copy)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
        }
    }
}

/// Which HTTP status codes are retryable.
pub fn is_retryable_http_status(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

/// Which errors should never be retried (auth, structural, cancellation).
pub fn is_non_retryable_http_status(status: u16) -> bool {
    matches!(status, 401 | 403)
}

/// Computes the next delay with capped exponential backoff + simple
/// deterministic jitter based on attempt number. In production this would
/// use random jitter; the deterministic variant is sufficient for S4 testing.
pub fn backoff_delay(config: &RetryConfig, attempt: u32) -> Duration {
    let base = config.base_delay.as_millis() as u64;
    let shift = (attempt.saturating_sub(1)).min(63);
    let exp = base.saturating_mul(1u64 << shift);
    let capped = exp.min(config.max_delay.as_millis() as u64);
    // Deterministic jitter: ±25% based on attempt parity
    let jitter = capped / 8;
    let delay = if attempt % 2 == 0 {
        capped.saturating_add(jitter)
    } else {
        capped.saturating_sub(jitter)
    };
    Duration::from_millis(delay.max(100))
}

/// Outcome of a provider request attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptOutcome {
    Success,
    Retryable(String),
    Fatal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_increases_with_attempts() {
        let config = RetryConfig::default();
        let d1 = backoff_delay(&config, 1);
        let d2 = backoff_delay(&config, 2);
        let d3 = backoff_delay(&config, 3);
        assert!(d2 > d1);
        assert!(d3 > d2);
    }

    #[test]
    fn backoff_respects_max_delay() {
        let config = RetryConfig {
            max_delay: Duration::from_millis(500),
            ..Default::default()
        };
        let delay = backoff_delay(&config, 10);
        assert!(delay <= Duration::from_millis(700)); // jitter can exceed slightly
    }

    #[test]
    fn http_429_is_retryable() {
        assert!(is_retryable_http_status(429));
    }

    #[test]
    fn http_401_is_not_retryable() {
        assert!(!is_retryable_http_status(401));
        assert!(is_non_retryable_http_status(401));
    }
}

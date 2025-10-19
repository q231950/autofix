use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A simple token bucket rate limiter for tracking API token usage
///
/// This prevents hitting Anthropic's rate limits by tracking token usage
/// over a rolling time window and delaying requests when necessary.
pub struct RateLimiter {
    state: Mutex<RateLimiterState>,
    tokens_per_minute: usize,
    enabled: bool,
}

struct RateLimiterState {
    tokens_used: usize,
    window_start: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter with the specified tokens per minute limit
    ///
    /// # Arguments
    /// * `tokens_per_minute` - Maximum tokens allowed per minute (default: 50000)
    /// * `enabled` - Whether rate limiting is enabled
    pub fn new(tokens_per_minute: usize, enabled: bool) -> Self {
        Self {
            state: Mutex::new(RateLimiterState {
                tokens_used: 0,
                window_start: Instant::now(),
            }),
            tokens_per_minute,
            enabled,
        }
    }

    /// Check if a request with the given token count can proceed
    /// Returns the number of seconds to wait if the request should be delayed
    ///
    /// # Arguments
    /// * `estimated_tokens` - Estimated number of input tokens for the request
    ///
    /// # Returns
    /// * `Ok(())` - Request can proceed immediately
    /// * `Err(Duration)` - Request should wait for the specified duration
    pub fn check_and_wait(&self, estimated_tokens: usize) -> Result<(), Duration> {
        if !self.enabled {
            return Ok(());
        }

        let mut state = self.state.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(state.window_start);

        // Reset the window if a minute has passed
        if elapsed >= Duration::from_secs(60) {
            state.tokens_used = 0;
            state.window_start = now;
            return Ok(());
        }

        // Check if adding these tokens would exceed the limit
        if state.tokens_used + estimated_tokens > self.tokens_per_minute {
            // Calculate how long to wait for the window to reset
            let wait_duration = Duration::from_secs(60) - elapsed;
            return Err(wait_duration);
        }

        // Allow the request
        Ok(())
    }

    /// Record that tokens were used in a request
    /// Call this after making an API request
    ///
    /// # Arguments
    /// * `tokens_used` - Actual number of tokens used (from API response)
    pub fn record_usage(&self, tokens_used: usize) {
        if !self.enabled {
            return;
        }

        let mut state = self.state.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(state.window_start);

        // Reset window if needed
        if elapsed >= Duration::from_secs(60) {
            state.tokens_used = tokens_used;
            state.window_start = now;
        } else {
            state.tokens_used += tokens_used;
        }
    }

    /// Get current usage statistics
    ///
    /// # Returns
    /// * `(tokens_used, tokens_remaining, seconds_until_reset)`
    pub fn get_stats(&self) -> (usize, usize, u64) {
        let state = self.state.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(state.window_start);
        let seconds_until_reset = 60 - elapsed.as_secs();
        let tokens_remaining = self.tokens_per_minute.saturating_sub(state.tokens_used);

        (state.tokens_used, tokens_remaining, seconds_until_reset)
    }

    /// Create a rate limiter from environment variables
    ///
    /// Reads:
    /// * `ANTHROPIC_RATE_LIMIT_TPM` - Tokens per minute limit (default: 50000)
    /// * `ANTHROPIC_RATE_LIMIT_ENABLED` - Enable rate limiting (default: true)
    pub fn from_env() -> Self {
        let tokens_per_minute = std::env::var("ANTHROPIC_RATE_LIMIT_TPM")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50000);

        let enabled = std::env::var("ANTHROPIC_RATE_LIMIT_ENABLED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        println!(
            "⏱️  Rate limiter configured: {} tokens/minute ({})",
            tokens_per_minute,
            if enabled { "enabled" } else { "disabled" }
        );

        Self::new(tokens_per_minute, enabled)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(50000, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(1000, true);
        assert!(limiter.check_and_wait(500).is_ok());
        limiter.record_usage(500);
        assert!(limiter.check_and_wait(400).is_ok());
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(1000, true);
        limiter.record_usage(900);
        let result = limiter.check_and_wait(200);
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limiter_disabled() {
        let limiter = RateLimiter::new(1000, false);
        limiter.record_usage(900);
        assert!(limiter.check_and_wait(1000).is_ok());
    }
}

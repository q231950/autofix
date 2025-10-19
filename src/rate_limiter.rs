use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A rolling window rate limiter for tracking API token usage
///
/// This prevents hitting Anthropic's rate limits by tracking actual token usage
/// from API responses over a rolling 60-second window and delaying requests when necessary.
pub struct RateLimiter {
    state: Mutex<RateLimiterState>,
    tokens_per_minute: usize,
    enabled: bool,
    verbose: bool,
}

struct RateLimiterState {
    // Rolling window of (timestamp, tokens_used) entries
    usage_history: VecDeque<(Instant, usize)>,
}

impl RateLimiter {
    /// Create a new rate limiter with the specified tokens per minute limit
    ///
    /// # Arguments
    /// * `tokens_per_minute` - Maximum tokens allowed per minute (default: 50000)
    /// * `enabled` - Whether rate limiting is enabled
    /// * `verbose` - Whether to print verbose debug information
    pub fn new(tokens_per_minute: usize, enabled: bool, verbose: bool) -> Self {
        Self {
            state: Mutex::new(RateLimiterState {
                usage_history: VecDeque::new(),
            }),
            tokens_per_minute,
            enabled,
            verbose,
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
        let window_start = now - Duration::from_secs(60);

        // Remove entries older than 60 seconds
        while let Some(&(timestamp, _)) = state.usage_history.front() {
            if timestamp < window_start {
                state.usage_history.pop_front();
            } else {
                break;
            }
        }

        // Calculate tokens used in the last 60 seconds
        let tokens_in_window: usize = state.usage_history.iter().map(|(_, tokens)| tokens).sum();

        // Check if adding these estimated tokens would exceed the limit
        if tokens_in_window + estimated_tokens > self.tokens_per_minute {
            // Find the oldest entry to determine when it will expire
            if let Some(&(oldest_timestamp, oldest_tokens)) = state.usage_history.front() {
                // Calculate when enough tokens will be freed up
                let time_until_oldest_expires = oldest_timestamp + Duration::from_secs(60) - now;

                // If freeing the oldest entry would be enough, wait for it
                if tokens_in_window - oldest_tokens + estimated_tokens <= self.tokens_per_minute {
                    return Err(time_until_oldest_expires);
                }

                // Otherwise, we need to wait longer - find when enough tokens free up
                let mut cumulative_freed = 0;
                for &(timestamp, tokens) in state.usage_history.iter() {
                    cumulative_freed += tokens;
                    if tokens_in_window - cumulative_freed + estimated_tokens
                        <= self.tokens_per_minute
                    {
                        let wait_time = timestamp + Duration::from_secs(60) - now;
                        return Err(wait_time);
                    }
                }

                // Worst case: wait 60 seconds for full window reset
                return Err(Duration::from_secs(60));
            }

            // No history but still over limit? Wait 60 seconds
            return Err(Duration::from_secs(60));
        }

        Ok(())
    }

    /// Record actual token usage from an API response
    /// Call this after receiving an API response with usage information
    ///
    /// # Arguments
    /// * `tokens_used` - Actual number of input tokens used (from response.usage.input_tokens)
    pub fn record_usage(&self, tokens_used: usize) {
        if !self.enabled {
            return;
        }

        let mut state = self.state.lock().unwrap();
        let now = Instant::now();

        // Add this usage to the rolling window
        state.usage_history.push_back((now, tokens_used));

        // Clean up old entries (older than 60 seconds)
        let window_start = now - Duration::from_secs(60);
        while let Some(&(timestamp, _)) = state.usage_history.front() {
            if timestamp < window_start {
                state.usage_history.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get current usage statistics
    ///
    /// # Returns
    /// * `(tokens_used, tokens_remaining, seconds_until_oldest_expires)`
    pub fn get_stats(&self) -> (usize, usize, u64) {
        let mut state = self.state.lock().unwrap();
        let now = Instant::now();
        let window_start = now - Duration::from_secs(60);

        // Clean up old entries
        while let Some(&(timestamp, _)) = state.usage_history.front() {
            if timestamp < window_start {
                state.usage_history.pop_front();
            } else {
                break;
            }
        }

        // Calculate tokens used in the last 60 seconds
        let tokens_used: usize = state.usage_history.iter().map(|(_, tokens)| tokens).sum();
        let tokens_remaining = self.tokens_per_minute.saturating_sub(tokens_used);

        // Calculate when the oldest entry will expire
        let seconds_until_reset = if let Some(&(oldest_timestamp, _)) = state.usage_history.front()
        {
            let expires_at = oldest_timestamp + Duration::from_secs(60);
            expires_at.saturating_duration_since(now).as_secs()
        } else {
            0
        };

        (tokens_used, tokens_remaining, seconds_until_reset)
    }

    /// Create a rate limiter from environment variables
    ///
    /// Reads:
    /// * `ANTHROPIC_RATE_LIMIT_TPM` - Tokens per minute limit (default: 50000)
    /// * `ANTHROPIC_RATE_LIMIT_ENABLED` - Enable rate limiting (default: true)
    ///
    /// # Arguments
    /// * `verbose` - Whether to print verbose debug information
    pub fn from_env(verbose: bool) -> Self {
        let tokens_per_minute = std::env::var("ANTHROPIC_RATE_LIMIT_TPM")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50000);

        let enabled = std::env::var("ANTHROPIC_RATE_LIMIT_ENABLED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        if verbose {
            println!(
                "  [DEBUG] Rate limiter configured: {} tokens/minute ({})",
                tokens_per_minute,
                if enabled { "enabled" } else { "disabled" }
            );
        }

        Self::new(tokens_per_minute, enabled, verbose)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(50000, true, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(1000, true, false);
        // Check if first request can proceed (no history yet)
        assert!(limiter.check_and_wait(500).is_ok());
        // Record actual usage from API response
        limiter.record_usage(500);
        // Check if second request can proceed
        assert!(limiter.check_and_wait(400).is_ok());
        // Record second usage
        limiter.record_usage(400);
        // Verify stats
        let (used, remaining, _) = limiter.get_stats();
        assert_eq!(used, 900);
        assert_eq!(remaining, 100);
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(1000, true, false);
        // Record 900 tokens used
        limiter.record_usage(900);
        // Next request would exceed limit
        let result = limiter.check_and_wait(200);
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limiter_disabled() {
        let limiter = RateLimiter::new(1000, false, false);
        // When disabled, all requests should succeed
        assert!(limiter.check_and_wait(900).is_ok());
        assert!(limiter.check_and_wait(1000).is_ok());
    }

    #[test]
    fn test_rate_limiter_rolling_window() {
        let limiter = RateLimiter::new(1000, true, false);
        // Record some usage
        limiter.record_usage(500);
        // Verify current usage
        let (used, remaining, _) = limiter.get_stats();
        assert_eq!(used, 500);
        assert_eq!(remaining, 500);

        // Can still use 400 more
        assert!(limiter.check_and_wait(400).is_ok());
        limiter.record_usage(400);

        // Now at 900, can't use 200 more
        assert!(limiter.check_and_wait(200).is_err());
    }
}

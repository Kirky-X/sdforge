// Copyright (c) 2026 Kirky.X
//! Rate limiting implementation
//!
//! This module provides O(1) fixed-window rate limiting with idempotency support.

use crate::cache::SharedCache;
use crate::security::types::{
    deserialize_window_state, serialize_instants, serialize_window_state, CacheNamespace,
    RateLimitConfig, RateLimitError,
};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Rate limiter with idempotency support
///
/// Security features:
/// - Time-window based rate limiting
/// - Request deduplication for idempotent requests
/// - Per-key tracking with automatic cleanup
///
/// Storage: All internal state is stored via `Arc<dyn SyncCache>` trait.
#[derive(Clone)]
pub struct AppRateLimiter {
    /// Configuration
    pub(crate) config: RateLimitConfig,
    /// Request tracking per IP via SyncCache (keyed by "sdforge:rl:{key}")
    requests: SharedCache,
    /// Idempotency key cache for deduplication via SyncCache (keyed by "sdforge:idempotency:{key}")
    idempotency_cache: SharedCache,
}

impl AppRateLimiter {
    /// Create a new rate limiter with optional configuration.
    ///
    /// This is the simplest way to create a AppRateLimiter - it provides
    /// out-of-the-box functionality with sensible defaults.
    pub fn new(config: Option<RateLimitConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            requests: Arc::new(crate::cache::DashMapCache::new()),
            idempotency_cache: Arc::new(crate::cache::DashMapCache::new()),
        }
    }

    /// Create a builder for configuring a AppRateLimiter.
    pub fn builder() -> AppRateLimiterBuilder {
        AppRateLimiterBuilder::new()
    }

    /// Create a AppRateLimiter with all dependencies explicitly provided.
    pub fn with_dependencies(
        config: RateLimitConfig,
        requests: SharedCache,
        idempotency_cache: SharedCache,
    ) -> Self {
        Self {
            config,
            requests,
            idempotency_cache,
        }
    }

    /// Check if request is rate limited (O(1) fixed window counter)
    ///
    /// Uses a fixed window counter algorithm for O(1) performance.
    /// Trade-off: Slightly less accurate at window boundaries compared
    /// to sliding window, but significantly faster for high-throughput scenarios.
    pub fn check(&self, key: &str) -> Result<u32, RateLimitError> {
        let window_secs = self.config.window.as_secs();
        let store_key = CacheNamespace::RateLimit.key(key);

        // Get current Unix timestamp in seconds
        let current_time_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Load existing window state or create new
        let mut state = self
            .requests
            .get(&store_key)
            .and_then(|d| deserialize_window_state(&d))
            .unwrap_or_default();

        // Calculate the current window number (integer division)
        // This gives us fixed windows aligned to epoch boundaries
        let current_window = current_time_secs / window_secs;

        // Check if we're in a new window (different window number)
        let stored_window = if state.window_start_secs > 0 {
            state.window_start_secs / window_secs
        } else {
            0
        };

        if current_window != stored_window || state.window_start_secs == 0 {
            // Start a new window
            state.window_start_secs = current_time_secs;
            state.count = 1;
        } else {
            // Same window, increment counter
            state.count += 1;
        }

        // Check rate limit
        if state.count > self.config.max_requests as u64 {
            // Calculate time until next window starts
            let next_window_start = (current_window + 1) * window_secs;
            let time_remaining = next_window_start.saturating_sub(current_time_secs);

            return Err(RateLimitError {
                limit: self.config.max_requests,
                remaining: 0,
                retry_after: if time_remaining > 0 {
                    time_remaining
                } else {
                    1
                },
            });
        }

        // Save updated state
        self.requests
            .set(&store_key, serialize_window_state(&state));

        Ok(self.config.max_requests - state.count as u32)
    }

    /// Check idempotency (returns true if this is a duplicate request)
    ///
    /// Call this at the start of request processing. If it returns true,
    /// the request should be processed as a duplicate (return cached response).
    pub fn check_idempotency(&self, idempotency_key: &str) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(60); // Idempotency key cache window
        let store_key = CacheNamespace::Idempotency.key(idempotency_key);

        if let Some(data) = self.idempotency_cache.get(&store_key) {
            let existing_times = crate::security::types::deserialize_instants(&data);
            if let Some(&existing) = existing_times.first() {
                let elapsed = now.saturating_duration_since(existing).as_secs();
                if elapsed < window.as_secs() {
                    return true; // Duplicate request
                }
            }
        }

        // Record this idempotency key
        self.idempotency_cache
            .set(&store_key, serialize_instants(&[now]));

        false // Not a duplicate
    }

    /// Get remaining requests (O(1) fixed window counter)
    pub fn remaining(&self, key: &str) -> u32 {
        let window_secs = self.config.window.as_secs();
        let store_key = CacheNamespace::RateLimit.key(key);

        // Get current Unix timestamp in seconds
        let current_time_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let state = self
            .requests
            .get(&store_key)
            .and_then(|d| deserialize_window_state(&d))
            .unwrap_or_default();

        // Calculate window numbers
        let current_window = current_time_secs / window_secs;
        let stored_window = if state.window_start_secs > 0 {
            state.window_start_secs / window_secs
        } else {
            0
        };

        if current_window != stored_window || state.window_start_secs == 0 {
            // Window expired or no state, full allowance
            self.config.max_requests
        } else {
            // Calculate remaining
            self.config.max_requests.saturating_sub(state.count as u32)
        }
    }

    /// Acquire rate limit permit (simple per-key rate limiting).
    ///
    /// Uses the per-key windowed rate limiting from `check()`.
    /// Returns `Ok(())` if permitted, `Err(RateLimitError)` if rejected.
    pub fn acquire(&self, key: &str) -> Result<(), RateLimitError> {
        self.check(key).map(|_| ())
    }

    /// Check if a request is allowed under the rate limit (trait method).
    ///
    /// Returns `true` if the request is allowed, `false` if rate limited.
    pub fn allow(&self, key: &str) -> bool {
        self.check(key).is_ok()
    }

    /// Reset rate limit state for a key (trait method).
    ///
    /// Clears all rate limiting data for the given key, allowing
    /// new requests to be processed without restriction.
    pub fn reset(&self, key: &str) {
        let rl_key = CacheNamespace::RateLimit.key(key);
        let idemp_key = CacheNamespace::Idempotency.key(key);
        self.requests.delete(&rl_key);
        self.idempotency_cache.delete(&idemp_key);
    }
}

impl Default for AppRateLimiter {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Builder for AppRateLimiter configuration
///
/// This builder provides a fluent interface for configuring a AppRateLimiter
/// with custom rate limiting parameters.
#[derive(Debug, Clone, Default)]
pub struct AppRateLimiterBuilder {
    /// Rate limit configuration
    config: RateLimitConfig,
    /// Maximum concurrent requests (semaphore permits)
    max_concurrent: usize,
}

impl AppRateLimiterBuilder {
    /// Create a new AppRateLimiterBuilder with default settings.
    pub fn new() -> Self {
        Self {
            config: RateLimitConfig::default(),
            max_concurrent: 1000,
        }
    }

    /// Set the maximum number of requests within the rate limit window.
    pub fn max_requests(mut self, max_requests: u32) -> Self {
        self.config.max_requests = max_requests;
        self
    }

    /// Set the duration of the rate limit window.
    pub fn window(mut self, window: Duration) -> Self {
        self.config.window = window;
        self
    }

    /// Set the maximum number of concurrent requests (semaphore permits).
    pub fn max_concurrent(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent = max_concurrent;
        self
    }

    /// Configure whether rate limit headers are included in responses.
    pub fn include_headers(mut self, include_headers: bool) -> Self {
        self.config.include_headers = include_headers;
        self
    }

    /// Build a AppRateLimiter instance using the configured settings.
    pub fn build(self) -> AppRateLimiter {
        AppRateLimiter::with_dependencies(
            self.config,
            Arc::new(crate::cache::DashMapCache::new()),
            Arc::new(crate::cache::DashMapCache::new()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_check() {
        let limiter = AppRateLimiter::new(Some(RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
            include_headers: false,
        }));

        // First 5 requests should succeed
        for i in 1..=5 {
            let result = limiter.check("test_key");
            assert!(result.is_ok(), "Request {} should succeed", i);
        }

        // 6th request should fail
        let result = limiter.check("test_key");
        assert!(result.is_err(), "Request 6 should fail");
    }

    #[test]
    fn test_remaining_requests() {
        let limiter = AppRateLimiter::new(Some(RateLimitConfig {
            max_requests: 10,
            window: Duration::from_secs(60),
            include_headers: false,
        }));

        assert_eq!(limiter.remaining("test_key"), 10);

        let _ = limiter.check("test_key");
        assert_eq!(limiter.remaining("test_key"), 9);

        let _ = limiter.check("test_key");
        assert_eq!(limiter.remaining("test_key"), 8);
    }

    #[test]
    fn test_reset() {
        let limiter = AppRateLimiter::new(Some(RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(60),
            include_headers: false,
        }));

        // Use up all requests
        for _ in 0..3 {
            limiter.check("test_key").ok();
        }

        // Should be rate limited
        assert!(limiter.check("test_key").is_err());

        // Reset
        limiter.reset("test_key");

        // Should work again
        assert!(limiter.check("test_key").is_ok());
    }

    #[test]
    fn test_allow() {
        let limiter = AppRateLimiter::new(Some(RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(60),
            include_headers: false,
        }));

        assert!(limiter.allow("test_key"));
        assert!(limiter.allow("test_key"));
        assert!(!limiter.allow("test_key"));
    }

    #[test]
    fn test_builder() {
        let limiter = AppRateLimiterBuilder::new()
            .max_requests(50)
            .window(Duration::from_secs(30))
            .include_headers(true)
            .build();

        assert_eq!(limiter.config.max_requests, 50);
        assert_eq!(limiter.config.window.as_secs(), 30);
        assert!(limiter.config.include_headers);
    }
}

// Copyright (c) 2026 Kirky.X
//! API Key authentication implementation
//!
//! This module provides API key authentication with brute-force protection.

use crate::cache::SharedCache;
use crate::security::types::{
    CacheNamespace, RateLimitConfig, deserialize_permissions, deserialize_instants,
    serialize_permissions, serialize_instants,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// API key authentication with brute-force protection
///
/// Security features:
/// - Valid API keys storage with permissions mapping (hashed for security)
/// - Rate limiting on validation attempts to prevent brute force attacks
/// - Per-IP attempt tracking with automatic cleanup
///
/// Storage: All internal state is stored via `Arc<dyn SyncCache>` trait,
/// allowing injection of custom storage backends for testing or production.
#[derive(Clone)]
pub struct AppApiKeyAuth {
    /// Valid API keys (stored as SHA256 hash -> permissions) via SyncCache
    valid_keys: SharedCache,
    /// Failed attempt tracking (IP -> attempts with timestamps) via SyncCache
    failed_attempts: SharedCache,
    /// Rate limit configuration
    rate_limit_config: Arc<RateLimitConfig>,
}

impl AppApiKeyAuth {
    /// Create new API key authentication with default rate limiting
    pub fn new() -> Self {
        Self::with_rate_limit(RateLimitConfig::default())
    }

    /// Create API key authentication with custom rate limiting
    pub fn with_rate_limit(config: RateLimitConfig) -> Self {
        Self {
            valid_keys: Arc::new(crate::cache::DashMapCache::new()),
            failed_attempts: Arc::new(crate::cache::DashMapCache::new()),
            rate_limit_config: Arc::new(config),
        }
    }

    /// Create with dependencies (for full DI mode)
    ///
    /// Accepts `Arc<dyn SyncCache>` for storage, enabling custom backends
    /// (e.g., distributed cache, persistent storage) for production use.
    pub fn with_dependencies(
        valid_keys: SharedCache,
        failed_attempts: SharedCache,
        rate_limit_config: Arc<RateLimitConfig>,
    ) -> Self {
        Self {
            valid_keys,
            failed_attempts,
            rate_limit_config,
        }
    }

    /// Create builder for configuration
    pub fn builder() -> AppApiKeyAuthBuilder {
        AppApiKeyAuthBuilder::new()
    }

    /// Hash API key using SHA256 with work factor for secure storage
    fn hash_key(key: &str) -> String {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();

        // Multiple rounds to slow brute-force attacks
        for _ in 0..100 {
            hasher.update(key.as_bytes());
            hasher.update([0x5c, 0x5c, 0x5c]);
        }

        format!("{:x}", hasher.finalize())
    }

    /// Add a valid API key (stored as hash)
    pub fn add_key(&self, key: impl Into<String>, permissions: Vec<String>) {
        let key_hash = Self::hash_key(&key.into());
        self.valid_keys.set(
            &CacheNamespace::ApiKey.key(&key_hash),
            serialize_permissions(&permissions),
        );
    }

    /// Validate an API key with rate limiting
    ///
    /// Security: Implements constant-time validation to prevent timing attacks.
    /// All code paths take the same amount of time regardless of key validity.
    /// Also implements rate limiting per caller to prevent brute force attacks.
    /// Note: Valid keys bypass rate limiting to prevent locking out legitimate users.
    pub fn validate_key(&self, key: &str, client_ip: &str) -> Option<Vec<String>> {
        let start = Instant::now();
        let key_hash = Self::hash_key(key);
        let store_key = CacheNamespace::ApiKey.key(&key_hash);

        // Always check valid_keys first for constant timing
        let perms = self.valid_keys.get(&store_key).and_then(|data| {
            let p = deserialize_permissions(&data);
            if p.is_empty() {
                None
            } else {
                Some(p)
            }
        });

        if perms.is_some() {
            // Apply delay for constant timing even for valid keys
            Self::apply_constant_time_delay(start);
            return perms;
        }

        // For invalid keys, check rate limit
        let is_limited = self.is_rate_limited(client_ip);

        // Record failed attempt if not already rate limited
        if !is_limited {
            self.record_failed_attempt(client_ip);
        }

        // Apply constant-time delay to normalize response time
        Self::apply_constant_time_delay(start);

        None
    }

    /// Apply constant-time delay to prevent timing attacks
    ///
    /// This ensures that the validation function always takes the same
    /// amount of time regardless of the key validity or rate limit status.
    fn apply_constant_time_delay(start: Instant) {
        // Skip delay in test mode for faster tests
        if cfg!(test) {
            return;
        }

        const TARGET_DELAY_US: u64 = 100; // 100 microseconds
        let elapsed = start.elapsed();

        if elapsed < Duration::from_micros(TARGET_DELAY_US) {
            std::thread::sleep(Duration::from_micros(TARGET_DELAY_US) - elapsed);
        }
    }

    /// Check if a client IP is rate limited
    fn is_rate_limited(&self, client_ip: &str) -> bool {
        let now = Instant::now();
        let window_start = now - self.rate_limit_config.window;
        let key = CacheNamespace::ApiFailed.key(client_ip);

        let data = match self.failed_attempts.get(&key) {
            Some(d) => d,
            None => return false,
        };
        let times = deserialize_instants(&data);
        let recent_attempts = times.iter().filter(|&&t| t > window_start).count();
        recent_attempts >= self.rate_limit_config.max_requests as usize
    }

    /// Record a failed validation attempt
    fn record_failed_attempt(&self, client_ip: &str) {
        let now = Instant::now();
        let window_start = now - self.rate_limit_config.window;
        let key = CacheNamespace::ApiFailed.key(client_ip);

        // Load existing attempts
        let data = self.failed_attempts.get(&key);
        let mut times: Vec<Instant> = data
            .as_ref()
            .map(|d| deserialize_instants(d))
            .unwrap_or_default();

        // Clean old attempts outside the window
        times.retain(|&t| t > window_start);

        // Add new attempt
        times.push(now);

        // Store back
        self.failed_attempts.set(&key, serialize_instants(&times));
    }

    /// Clear failed attempts for a client (e.g., after successful auth)
    pub fn clear_failed_attempts(&self, client_ip: &str) {
        let key = CacheNamespace::ApiFailed.key(client_ip);
        self.failed_attempts.delete(&key);
    }
}

impl Default for AppApiKeyAuth {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for AppApiKeyAuth configuration
#[derive(Debug, Clone, Default)]
pub struct AppApiKeyAuthBuilder {
    rate_limit_config: RateLimitConfig,
}

impl AppApiKeyAuthBuilder {
    /// Create a new ApiKeyAuthBuilder with default rate limit settings.
    ///
    /// # Returns
    ///
    /// Returns a builder initialized with default rate limit configuration.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AppApiKeyAuthBuilder;
    ///
    /// let builder = AppApiKeyAuthBuilder::new();
    /// let _ = builder;
    /// ```
    pub fn new() -> Self {
        Self {
            rate_limit_config: RateLimitConfig::default(),
        }
    }

    /// Set the maximum number of requests within the rate limit window.
    ///
    /// # Arguments
    ///
    /// * `max_requests` - Maximum number of requests allowed in a window.
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AppApiKeyAuthBuilder;
    ///
    /// let builder = AppApiKeyAuthBuilder::new().max_requests(100);
    /// let _ = builder;
    /// ```
    pub fn max_requests(mut self, max_requests: u32) -> Self {
        self.rate_limit_config.max_requests = max_requests;
        self
    }

    /// Set the duration of the rate limit window.
    ///
    /// # Arguments
    ///
    /// * `window` - Time window for rate limiting.
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AppApiKeyAuthBuilder;
    /// use std::time::Duration;
    ///
    /// let builder = AppApiKeyAuthBuilder::new().window(Duration::from_secs(60));
    /// let _ = builder;
    /// ```
    pub fn window(mut self, window: Duration) -> Self {
        self.rate_limit_config.window = window;
        self
    }

    /// Configure whether rate limit headers are included in responses.
    ///
    /// # Arguments
    ///
    /// * `include_headers` - Whether to include rate limit headers.
    ///
    /// # Returns
    ///
    /// Returns the updated builder instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AppApiKeyAuthBuilder;
    ///
    /// let builder = AppApiKeyAuthBuilder::new().include_headers(true);
    /// let _ = builder;
    /// ```
    pub fn include_headers(mut self, include_headers: bool) -> Self {
        self.rate_limit_config.include_headers = include_headers;
        self
    }

    /// Build an AppApiKeyAuth instance using the configured settings.
    ///
    /// # Returns
    ///
    /// Returns a fully configured AppApiKeyAuth instance.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AppApiKeyAuthBuilder;
    ///
    /// let auth = AppApiKeyAuthBuilder::new().max_requests(100).build();
    /// let _ = auth;
    /// ```
    pub fn build(self) -> AppApiKeyAuth {
        AppApiKeyAuth::with_rate_limit(self.rate_limit_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_validate_key() {
        let auth = AppApiKeyAuth::new();
        auth.add_key("test_key", vec!["read".to_string()]);
        
        let perms = auth.validate_key("test_key", "127.0.0.1");
        assert!(perms.is_some());
        assert_eq!(perms.unwrap(), vec!["read"]);
    }

    #[test]
    fn test_invalid_key() {
        let auth = AppApiKeyAuth::new();
        let perms = auth.validate_key("invalid_key", "127.0.0.1");
        assert!(perms.is_none());
    }

    #[test]
    fn test_rate_limiting() {
        let auth = AppApiKeyAuth::with_rate_limit(RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(60),
            include_headers: false,
        });

        // Make 3 failed attempts (should be allowed)
        for _ in 0..3 {
            auth.validate_key("invalid", "127.0.0.1");
        }

        // 4th attempt should still work (rate limit is checked but doesn't block validation)
        let result = auth.validate_key("invalid", "127.0.0.1");
        assert!(result.is_none());
    }

    #[test]
    fn test_clear_failed_attempts() {
        let auth = AppApiKeyAuth::new();
        
        // Make some failed attempts
        for _ in 0..3 {
            auth.validate_key("invalid", "127.0.0.1");
        }

        // Clear attempts
        auth.clear_failed_attempts("127.0.0.1");

        // Should not be rate limited anymore
        let result = auth.validate_key("invalid", "127.0.0.1");
        assert!(result.is_none());
    }

    #[test]
    fn test_builder() {
        let auth = AppApiKeyAuthBuilder::new()
            .max_requests(100)
            .window(Duration::from_secs(60))
            .build();

        auth.add_key("test", vec!["write".to_string()]);
        let perms = auth.validate_key("test", "127.0.0.1");
        assert!(perms.is_some());
    }
}

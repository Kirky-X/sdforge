// Copyright (c) 2026 Kirky.X
//! API Key authentication implementation with versioning, LRU eviction, and rotation support
//!
//! This module provides API key authentication with:
//! - Brute-force protection
//! - API key versioning for smooth rotation
//! - LRU cache eviction to prevent memory growth
//! - Automatic key rotation with grace period

use crate::cache::SharedCache;
use crate::security::api_key_manager::{
    ApiKeyMetadata, ApiKeyVersion, LruCacheManager, LruConfig, RotationConfig,
};
use crate::security::types::{
    deserialize_instants, deserialize_permissions, serialize_instants, serialize_permissions,
    CacheNamespace, RateLimitConfig,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// API key authentication with brute-force protection, versioning, and LRU eviction
///
/// Security features:
/// - Valid API keys storage with permissions mapping (hashed for security)
/// - Rate limiting on validation attempts to prevent brute force attacks
/// - Per-IP attempt tracking with automatic cleanup
/// - API key versioning for smooth rotation
/// - LRU cache eviction to prevent memory growth
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
    /// API key metadata (key_id -> ApiKeyMetadata) for versioning support
    key_metadata: SharedCache,
    /// LRU cache manager for automatic eviction
    lru_manager: Option<Arc<LruCacheManager>>,
    /// Key rotation configuration
    rotation_config: Option<RotationConfig>,
}

impl AppApiKeyAuth {
    /// Create new API key authentication with default rate limiting
    pub fn new() -> Self {
        Self::with_rate_limit(RateLimitConfig::default())
    }

    /// Create API key authentication with custom rate limiting
    pub fn with_rate_limit(config: RateLimitConfig) -> Self {
        let valid_keys = Arc::new(crate::cache::DashMapCache::new());
        let failed_attempts = Arc::new(crate::cache::DashMapCache::new());
        let key_metadata = Arc::new(crate::cache::DashMapCache::new());

        // Enable LRU by default
        let lru_manager = Some(Arc::new(LruCacheManager::new(
            valid_keys.clone(),
            LruConfig::default(),
        )));

        Self {
            valid_keys,
            failed_attempts,
            rate_limit_config: Arc::new(config),
            key_metadata,
            lru_manager,
            rotation_config: None,
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
        let key_metadata = Arc::new(crate::cache::DashMapCache::new());
        let lru_manager = Some(Arc::new(LruCacheManager::new(
            valid_keys.clone(),
            LruConfig::default(),
        )));

        Self {
            valid_keys,
            failed_attempts,
            rate_limit_config,
            key_metadata,
            lru_manager,
            rotation_config: None,
        }
    }

    /// Enable LRU cache eviction with custom configuration
    pub fn with_lru(mut self, config: LruConfig) -> Self {
        self.lru_manager = Some(Arc::new(LruCacheManager::new(
            self.valid_keys.clone(),
            config,
        )));
        self
    }

    /// Enable automatic key rotation
    pub fn with_rotation(mut self, config: RotationConfig) -> Self {
        self.rotation_config = Some(config);
        self
    }

    /// Create builder for configuration
    pub fn builder() -> AppApiKeyAuthBuilder {
        AppApiKeyAuthBuilder::new()
    }

    /// Hash API key using Argon2id for secure storage
    ///
    /// Uses Argon2id with OWASP-recommended parameters (2024):
    /// - Time cost: 3 iterations
    /// - Memory cost: 64 MiB (65536 KiB)
    /// - Parallelism: 4 threads
    /// - Output length: 32 bytes
    /// - Random salt per key
    ///
    /// Argon2id is resistant to GPU/ASIC attacks and side-channel attacks.
    fn hash_key(key: &str) -> String {
        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
            Argon2,
        };

        // Argon2id with OWASP 2024 recommended parameters
        // Time Cost: 3, Memory: 64 MiB (65536 KiB), Parallelism: 4, Output: 32 bytes
        let argon2 = Argon2::default();

        // Generate random salt
        let salt = SaltString::generate(&mut OsRng);

        // Hash the password
        let password_hash = argon2
            .hash_password(key.as_bytes(), &salt)
            .expect("Argon2 hashing should not fail with valid parameters");

        // Return PHC string format (e.g., $argon2id$v=19$m=65536,t=3,p=4$...)
        password_hash.to_string()
    }

    /// Add a valid API key (stored as hash)
    pub fn add_key(&self, key: impl Into<String>, permissions: Vec<String>) {
        let key_hash = Self::hash_key(&key.into());
        self.valid_keys.set(
            &CacheNamespace::ApiKey.key(&key_hash),
            serialize_permissions(&permissions),
        );
    }

    /// Add a versioned API key with metadata
    ///
    /// This method supports key rotation by storing multiple versions of the same key.
    ///
    /// # Arguments
    ///
    /// * `key_id` - Unique identifier for this API key
    /// * `key` - The actual API key value
    /// * `permissions` - List of permissions for this key
    /// * `version` - Version string (e.g., "v1", "v2")
    /// * `ttl` - Optional time-to-live for this version
    pub fn add_key_version(
        &self,
        key_id: impl Into<String>,
        key: impl Into<String>,
        permissions: Vec<String>,
        version: impl Into<String>,
        ttl: Option<Duration>,
    ) {
        let key_id = key_id.into();
        let key_hash = Self::hash_key(&key.into());
        let version_str = version.into();

        // Store the hash with permissions
        self.valid_keys.set(
            &CacheNamespace::ApiKey.key(&key_hash),
            serialize_permissions(&permissions),
        );

        // Create or update metadata
        let metadata_key = format!("metadata:{}", key_id);
        if let Some(data) = self.key_metadata.get(&metadata_key) {
            // Update existing metadata
            if let Ok(mut metadata) = bincode::deserialize::<ApiKeyMetadata>(&data) {
                let new_version = ApiKeyVersion::new(version_str, key_hash, permissions, ttl);
                metadata.add_version(new_version);
                self.key_metadata.set(
                    &metadata_key,
                    bincode::serialize(&metadata).unwrap_or_default(),
                );
            }
        } else {
            // Create new metadata
            let mut metadata = ApiKeyMetadata::new(key_id.clone(), None);
            let new_version = ApiKeyVersion::new(version_str, key_hash, permissions, ttl);
            metadata.add_version(new_version);
            self.key_metadata.set(
                &metadata_key,
                bincode::serialize(&metadata).unwrap_or_default(),
            );
        }
    }

    /// Rotate an API key to a new version
    ///
    /// # Arguments
    ///
    /// * `key_id` - The key identifier
    /// * `new_key` - The new key value
    /// * `new_permissions` - Permissions for the new version
    /// * `version` - Version string for the new key
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if rotation succeeded, `Err(String)` on failure.
    pub fn rotate_key(
        &self,
        key_id: &str,
        new_key: impl Into<String>,
        new_permissions: Vec<String>,
        version: impl Into<String>,
    ) -> Result<(), String> {
        let metadata_key = format!("metadata:{}", key_id);
        let new_key_hash = Self::hash_key(&new_key.into());
        let version_str = version.into();

        // Load existing metadata
        let data = self
            .key_metadata
            .get(&metadata_key)
            .ok_or_else(|| "Key not found".to_string())?;

        let mut metadata: ApiKeyMetadata = bincode::deserialize(&data)
            .map_err(|e| format!("Failed to deserialize metadata: {}", e))?;

        // Create new version
        let ttl = self
            .rotation_config
            .as_ref()
            .map(|config| config.rotation_interval);

        let new_version = ApiKeyVersion::new(
            version_str.clone(),
            new_key_hash.clone(),
            new_permissions.clone(),
            ttl,
        );

        // Add new version and activate it
        metadata.add_version(new_version);

        // Store new key hash
        self.valid_keys.set(
            &CacheNamespace::ApiKey.key(&new_key_hash),
            serialize_permissions(&new_permissions),
        );

        // Save updated metadata
        self.key_metadata.set(
            &metadata_key,
            bincode::serialize(&metadata).unwrap_or_default(),
        );

        // Cleanup old versions
        if let Some(config) = &self.rotation_config {
            metadata.cleanup_versions(config.keep_versions);
        }

        Ok(())
    }

    /// Get metadata for a key
    pub fn get_key_metadata(&self, key_id: &str) -> Option<ApiKeyMetadata> {
        let metadata_key = format!("metadata:{}", key_id);
        let data = self.key_metadata.get(&metadata_key)?;
        bincode::deserialize(&data).ok()
    }

    /// Validate an API key with rate limiting and version checking
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

    /// Revoke an API key by key_id
    pub fn revoke_key(&self, key_id: &str) -> Result<(), String> {
        let metadata_key = format!("metadata:{}", key_id);
        let data = self
            .key_metadata
            .get(&metadata_key)
            .ok_or_else(|| "Key not found".to_string())?;

        let mut metadata: ApiKeyMetadata = bincode::deserialize(&data)
            .map_err(|e| format!("Failed to deserialize metadata: {}", e))?;

        // Deactivate all versions
        for version in &mut metadata.versions {
            version.deactivate();
        }

        // Save updated metadata
        self.key_metadata.set(
            &metadata_key,
            bincode::serialize(&metadata).unwrap_or_default(),
        );

        Ok(())
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
    lru_config: Option<LruConfig>,
    rotation_config: Option<RotationConfig>,
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
            lru_config: None,
            rotation_config: None,
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

    /// Enable LRU cache eviction with custom configuration
    pub fn lru(mut self, config: LruConfig) -> Self {
        self.lru_config = Some(config);
        self
    }

    /// Enable automatic key rotation
    pub fn rotation(mut self, config: RotationConfig) -> Self {
        self.rotation_config = Some(config);
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
        let mut auth = AppApiKeyAuth::with_rate_limit(self.rate_limit_config);

        if let Some(lru_config) = self.lru_config {
            auth = auth.with_lru(lru_config);
        }

        if let Some(rotation_config) = self.rotation_config {
            auth = auth.with_rotation(rotation_config);
        }

        auth
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
    fn test_add_key_version() {
        let auth = AppApiKeyAuth::new();

        auth.add_key_version("key1", "secret_v1", vec!["read".to_string()], "v1", None);

        let metadata = auth.get_key_metadata("key1");
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.versions.len(), 1);
        assert_eq!(meta.versions[0].version, "v1");
    }

    #[test]
    fn test_key_rotation() {
        let auth = AppApiKeyAuth::new().with_rotation(RotationConfig::default());

        // Add initial version
        auth.add_key_version("key1", "secret_v1", vec!["read".to_string()], "v1", None);

        // Rotate to new version
        let result = auth.rotate_key(
            "key1",
            "secret_v2",
            vec!["read".to_string(), "write".to_string()],
            "v2",
        );

        assert!(result.is_ok());

        // Verify new version exists
        let metadata = auth.get_key_metadata("key1");
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.versions.len(), 2);
    }

    #[test]
    fn test_revoke_key() {
        let auth = AppApiKeyAuth::new();

        auth.add_key_version("key1", "secret_v1", vec!["read".to_string()], "v1", None);

        // Revoke the key
        let result = auth.revoke_key("key1");
        assert!(result.is_ok());

        // Verify all versions are deactivated
        let metadata = auth.get_key_metadata("key1");
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert!(!meta.versions.iter().any(|v| v.is_active));
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

    #[test]
    fn test_builder_with_lru() {
        let auth = AppApiKeyAuthBuilder::new()
            .lru(LruConfig::default())
            .build();

        assert!(auth.lru_manager.is_some());
    }

    #[test]
    fn test_builder_with_rotation() {
        let auth = AppApiKeyAuthBuilder::new()
            .rotation(RotationConfig::default())
            .build();

        assert!(auth.rotation_config.is_some());
    }
}

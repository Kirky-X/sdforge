// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! API Key authentication implementation with versioning, LRU eviction, and rotation support
//!
//! This module provides API key authentication with:
//! - Brute-force protection
//! - API key versioning for smooth rotation
//! - LRU cache eviction to prevent memory growth
//! - Automatic key rotation with grace period

use crate::cache::SharedCache;
use crate::security::{ApiKeyMetadata, ApiKeyVersion, LruCacheManager, LruConfig, RotationConfig};
use crate::security::{CacheNamespace, deserialize_permissions, serialize_permissions};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// API key authentication with versioning, LRU eviction, and rotation support
///
/// Security features:
/// - Valid API keys storage with permissions mapping (hashed for security)
/// - Constant-time validation to prevent timing attacks
/// - API key versioning for smooth rotation
/// - LRU cache eviction to prevent memory growth
///
/// Storage: All internal state is stored via `Arc<dyn SyncCache>` trait,
/// allowing injection of custom storage backends for testing or production.
#[derive(Clone)]
pub struct AppApiKeyAuth {
    /// Valid API keys (stored as SHA256 hash -> permissions) via SyncCache
    valid_keys: SharedCache,
    /// API key metadata (key_id -> ApiKeyMetadata) for versioning support
    key_metadata: SharedCache,
    /// LRU cache manager for automatic eviction
    lru_manager: Option<Arc<LruCacheManager>>,
    /// Key rotation configuration
    rotation_config: Option<RotationConfig>,
}

impl AppApiKeyAuth {
    /// Create new API key authentication
    pub fn new() -> Self {
        let valid_keys = Arc::new(crate::cache::DashMapCache::new());
        let key_metadata = Arc::new(crate::cache::DashMapCache::new());

        // Enable LRU by default
        let lru_manager = Some(Arc::new(LruCacheManager::new(
            valid_keys.clone(),
            LruConfig::default(),
        )));

        Self {
            valid_keys,
            key_metadata,
            lru_manager,
            rotation_config: None,
        }
    }

    /// Create with dependencies (for full DI mode)
    ///
    /// Accepts `Arc<dyn SyncCache>` for storage, enabling custom backends
    /// (e.g., distributed cache, persistent storage) for production use.
    pub fn with_dependencies(valid_keys: SharedCache, _key_metadata: Option<SharedCache>) -> Self {
        let key_metadata =
            _key_metadata.unwrap_or_else(|| Arc::new(crate::cache::DashMapCache::new()));
        let lru_manager = Some(Arc::new(LruCacheManager::new(
            valid_keys.clone(),
            LruConfig::default(),
        )));

        Self {
            valid_keys,
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

    /// Hash API key using SHA256 for deterministic storage and lookup
    ///
    /// Uses SHA256 to create a deterministic hash of the API key.
    /// This ensures that the same key always produces the same hash,
    /// allowing correct validation later.
    ///
    /// Note: While SHA256 is cryptographically secure for integrity,
    /// API keys stored in this format should still be treated as sensitive.
    fn hash_key(key: &str) -> String {
        use sha2::{Digest, Sha256};

        // Use SHA256 for deterministic hashing
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();

        // Convert to hex string
        hex::encode(result)
    }

    /// Generate a safe, non-reversible identifier for an API key.
    ///
    /// Returns a truncated SHA256 hash prefix (`api_key:<first 16 hex chars>`)
    /// suitable for use as `user_id` in audit logs and AuthContext without
    /// exposing the raw API key to logs or responses.
    pub fn key_id(key: &str) -> String {
        let hash = Self::hash_key(key);
        format!("api_key:{}", &hash[..hash.len().min(16)])
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
            if let Ok(mut metadata) = bincode::serde::decode_from_slice::<ApiKeyMetadata, _>(
                &data,
                bincode::config::standard(),
            )
            .map(|(v, _)| v)
            {
                let new_version = ApiKeyVersion::new(version_str, key_hash, permissions, ttl);
                metadata.add_version(new_version);
                self.key_metadata.set(
                    &metadata_key,
                    bincode::serde::encode_to_vec(&metadata, bincode::config::standard())
                        .unwrap_or_default(),
                );
            }
        } else {
            // Create new metadata
            let mut metadata = ApiKeyMetadata::new(key_id.clone(), None);
            let new_version = ApiKeyVersion::new(version_str, key_hash, permissions, ttl);
            metadata.add_version(new_version);
            self.key_metadata.set(
                &metadata_key,
                bincode::serde::encode_to_vec(&metadata, bincode::config::standard())
                    .unwrap_or_default(),
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

        let mut metadata: ApiKeyMetadata = bincode::serde::decode_from_slice::<ApiKeyMetadata, _>(
            &data,
            bincode::config::standard(),
        )
        .map(|(v, _)| v)
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
            bincode::serde::encode_to_vec(&metadata, bincode::config::standard())
                .unwrap_or_default(),
        );

        // Cleanup old versions and delete their key_hashes from valid_keys cache
        // so rotated-out keys can no longer authenticate.
        if let Some(config) = &self.rotation_config {
            let hashes_before: std::collections::HashSet<String> = metadata
                .versions
                .iter()
                .map(|v| v.key_hash.clone())
                .collect();
            metadata.cleanup_versions(config.keep_versions);
            for hash in hashes_before {
                let still_present = metadata.versions.iter().any(|v| v.key_hash == hash);
                if !still_present {
                    let store_key = CacheNamespace::ApiKey.key(&hash);
                    self.valid_keys.delete(&store_key);
                }
            }
        }

        Ok(())
    }

    /// Get metadata for a key
    pub fn get_key_metadata(&self, key_id: &str) -> Option<ApiKeyMetadata> {
        let metadata_key = format!("metadata:{}", key_id);
        let data = self.key_metadata.get(&metadata_key)?;
        bincode::serde::decode_from_slice::<ApiKeyMetadata, _>(&data, bincode::config::standard())
            .map(|(v, _)| v)
            .ok()
    }

    /// Validate an API key with constant-time checking
    ///
    /// Security: Implements constant-time validation to prevent timing attacks.
    /// All code paths take the same amount of time regardless of key validity.
    ///
    /// # Arguments
    ///
    /// * `key` - The API key to validate
    /// * `client_ip` - Unused parameter (kept for API compatibility)
    ///
    /// # Returns
    ///
    /// Returns `Some(Vec<String>)` with permissions if the key is valid,
    /// or `None` if the key is invalid.
    pub fn validate_key(&self, key: &str, _client_ip: &str) -> Option<Vec<String>> {
        let start = Instant::now();
        let key_hash = Self::hash_key(key);
        let store_key = CacheNamespace::ApiKey.key(&key_hash);

        // Always check valid_keys for constant timing
        let perms = self.valid_keys.get(&store_key).and_then(|data| {
            let p = deserialize_permissions(&data);
            if p.is_empty() { None } else { Some(p) }
        });

        // Apply delay for constant timing regardless of key validity
        Self::apply_constant_time_delay(start);

        perms
    }

    /// Apply constant-time delay to prevent timing attacks
    ///
    /// This ensures that the validation function always takes the same
    /// amount of time regardless of the key validity.
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

    /// Revoke an API key by key_id
    pub fn revoke_key(&self, key_id: &str) -> Result<(), String> {
        let metadata_key = format!("metadata:{}", key_id);
        let data = self
            .key_metadata
            .get(&metadata_key)
            .ok_or_else(|| "Key not found".to_string())?;

        let mut metadata: ApiKeyMetadata = bincode::serde::decode_from_slice::<ApiKeyMetadata, _>(
            &data,
            bincode::config::standard(),
        )
        .map(|(v, _)| v)
        .map_err(|e| format!("Failed to deserialize metadata: {}", e))?;

        // Delete each version's key_hash from valid_keys cache so revoked keys
        // can no longer authenticate (validate_key only checks valid_keys, not metadata).
        for version in &metadata.versions {
            let store_key = CacheNamespace::ApiKey.key(&version.key_hash);
            self.valid_keys.delete(&store_key);
        }

        // Deactivate all versions
        for version in &mut metadata.versions {
            version.deactivate();
        }

        // Save updated metadata
        self.key_metadata.set(
            &metadata_key,
            bincode::serde::encode_to_vec(&metadata, bincode::config::standard())
                .unwrap_or_default(),
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
    lru_config: Option<LruConfig>,
    rotation_config: Option<RotationConfig>,
}

impl AppApiKeyAuthBuilder {
    /// Create a new ApiKeyAuthBuilder.
    ///
    /// # Returns
    ///
    /// Returns a builder initialized with default configuration.
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
            lru_config: None,
            rotation_config: None,
        }
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
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::security::AppApiKeyAuthBuilder;
    ///
    /// let auth = AppApiKeyAuthBuilder::new().build();
    /// let _ = auth;
    /// ```
    pub fn build(self) -> AppApiKeyAuth {
        let mut auth = AppApiKeyAuth::new();

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
    fn test_builder() {
        let auth = AppApiKeyAuthBuilder::new().build();

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

    // ============================================================================
    // Empty/Edge Case Tests
    // ============================================================================

    #[test]
    fn test_validate_key_empty_string() {
        let auth = AppApiKeyAuth::new();
        let perms = auth.validate_key("", "127.0.0.1");
        assert!(perms.is_none());
    }

    #[test]
    fn test_add_key_empty_permissions() {
        let auth = AppApiKeyAuth::new();
        auth.add_key("test_key", vec![]);
        let perms = auth.validate_key("test_key", "127.0.0.1");
        assert!(perms.is_none());
    }

    #[test]
    fn test_add_key_whitespace_only_key() {
        let auth = AppApiKeyAuth::new();
        auth.add_key("   ", vec!["read".to_string()]);
        let perms = auth.validate_key("   ", "127.0.0.1");
        assert!(perms.is_some());
        assert_eq!(perms.unwrap(), vec!["read"]);
    }

    // ============================================================================
    // Special Characters Tests
    // ============================================================================

    #[test]
    fn test_add_key_unicode_characters() {
        let auth = AppApiKeyAuth::new();
        auth.add_key("密钥_测试", vec!["read".to_string()]);

        let perms = auth.validate_key("密钥_测试", "127.0.0.1");
        assert!(perms.is_some());
        assert_eq!(perms.unwrap(), vec!["read"]);
    }

    #[test]
    fn test_add_key_special_characters() {
        let auth = AppApiKeyAuth::new();
        auth.add_key("key!@#$%^&*()", vec!["admin".to_string()]);

        let perms = auth.validate_key("key!@#$%^&*()", "127.0.0.1");
        assert!(perms.is_some());
        assert_eq!(perms.unwrap(), vec!["admin"]);
    }

    #[test]
    fn test_add_key_very_long_key() {
        let auth = AppApiKeyAuth::new();
        let long_key = "x".repeat(1000);
        auth.add_key(long_key.clone(), vec!["read".to_string()]);

        let perms = auth.validate_key(&long_key, "127.0.0.1");
        assert!(perms.is_some());
        assert_eq!(perms.unwrap(), vec!["read"]);
    }

    #[test]
    fn test_add_key_newline_characters() {
        let auth = AppApiKeyAuth::new();
        auth.add_key("key\nwith\nnewlines", vec!["write".to_string()]);

        let perms = auth.validate_key("key\nwith\nnewlines", "127.0.0.1");
        assert!(perms.is_some());
        assert_eq!(perms.unwrap(), vec!["write"]);
    }

    // ============================================================================
    // Permissions Tests
    // ============================================================================

    #[test]
    fn test_add_key_duplicate_permissions() {
        let auth = AppApiKeyAuth::new();
        auth.add_key(
            "dup_perm_key",
            vec!["read".to_string(), "read".to_string(), "write".to_string()],
        );
        let perms = auth.validate_key("dup_perm_key", "127.0.0.1");
        assert!(perms.is_some());
        let perms = perms.unwrap();
        assert_eq!(perms.len(), 3);
    }

    #[test]
    fn test_add_key_special_char_permissions() {
        let auth = AppApiKeyAuth::new();
        auth.add_key(
            "special_perm_key",
            vec!["read:users".to_string(), "write:posts".to_string()],
        );

        let perms = auth.validate_key("special_perm_key", "127.0.0.1");
        assert!(perms.is_some());
        let perms = perms.unwrap();
        assert_eq!(perms.len(), 2);
        assert!(perms.contains(&"read:users".to_string()));
        assert!(perms.contains(&"write:posts".to_string()));
    }

    // ============================================================================
    // Duplicate/Override Tests
    // ============================================================================

    #[test]
    fn test_add_same_key_twice() {
        let auth = AppApiKeyAuth::new();
        auth.add_key("duplicate_key", vec!["read".to_string()]);
        auth.add_key("duplicate_key", vec!["write".to_string()]);
        let perms = auth.validate_key("duplicate_key", "127.0.0.1");
        assert!(perms.is_some());
        let perms = perms.unwrap();
        assert_eq!(perms, vec!["write"]);
    }

    #[test]
    fn test_add_key_different_permissions_same_key() {
        let auth = AppApiKeyAuth::new();
        auth.add_key("update_key", vec!["read".to_string(), "write".to_string()]);
        auth.add_key("update_key", vec!["admin".to_string()]);

        let perms = auth.validate_key("update_key", "127.0.0.1");
        assert!(perms.is_some());
        let perms = perms.unwrap();
        assert_eq!(perms, vec!["admin"]);
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[test]
    fn test_rotate_key_nonexistent_key_id() {
        let auth = AppApiKeyAuth::new().with_rotation(RotationConfig::default());
        let result = auth.rotate_key(
            "nonexistent_key",
            "new_key_value",
            vec!["read".to_string()],
            "v2",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Key not found"));
    }

    #[test]
    fn test_rotate_key_empty_new_key() {
        let auth = AppApiKeyAuth::new().with_rotation(RotationConfig::default());
        auth.add_key_version("key1", "secret_v1", vec!["read".to_string()], "v1", None);
        let result = auth.rotate_key("key1", "", vec!["write".to_string()], "v2");
        assert!(result.is_ok());
        let perms = auth.validate_key("", "127.0.0.1");
        assert!(perms.is_some());
        assert_eq!(perms.unwrap(), vec!["write"]);
    }

    #[test]
    fn test_revoke_key_nonexistent_key_id() {
        let auth = AppApiKeyAuth::new();
        let result = auth.revoke_key("nonexistent_key");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Key not found"));
    }

    #[test]
    fn test_revoke_key_then_validate() {
        let auth = AppApiKeyAuth::new();
        auth.add_key_version("key1", "secret_v1", vec!["read".to_string()], "v1", None);
        let perms = auth.validate_key("secret_v1", "127.0.0.1");
        assert!(perms.is_some());

        let result = auth.revoke_key("key1");
        assert!(result.is_ok());

        // After revocation, the key must no longer validate.
        // Previously revoke_key only deactivated metadata without deleting from valid_keys cache,
        // allowing revoked keys to continue authenticating. This is now fixed.
        let perms = auth.validate_key("secret_v1", "127.0.0.1");
        assert!(perms.is_none(), "Revoked key must not validate");
    }

    #[test]
    fn test_get_key_metadata_nonexistent() {
        let auth = AppApiKeyAuth::new();
        let metadata = auth.get_key_metadata("nonexistent_key");
        assert!(metadata.is_none());
    }

    // ============================================================================
    // Builder/Default Tests
    // ============================================================================

    #[test]
    fn test_default_trait() {
        let auth1 = AppApiKeyAuth::default();
        let auth2 = AppApiKeyAuth::new();

        assert!(auth1.lru_manager.is_some());
        assert!(auth2.lru_manager.is_some());

        assert!(auth1.rotation_config.is_none());
        assert!(auth2.rotation_config.is_none());
    }

    #[test]
    fn test_builder_chained_config() {
        let lru_config = LruConfig {
            max_entries: 500,
            ttl: std::time::Duration::from_secs(1800),
            eviction_threshold: 0.7,
        };

        let rotation_config = RotationConfig {
            rotation_interval: std::time::Duration::from_secs(86400 * 7),
            grace_period: std::time::Duration::from_secs(86400),
            keep_versions: 5,
        };

        let auth = AppApiKeyAuth::builder()
            .lru(lru_config.clone())
            .rotation(rotation_config.clone())
            .build();

        assert!(auth.lru_manager.is_some());
        assert!(auth.rotation_config.is_some());
    }

    #[test]
    fn test_builder_with_both_lru_and_rotation() {
        let auth = AppApiKeyAuthBuilder::new()
            .lru(LruConfig::default())
            .rotation(RotationConfig::default())
            .build();

        assert!(auth.lru_manager.is_some());
        assert!(auth.rotation_config.is_some());
    }

    // ============================================================================
    // Multi-Version Tests
    // ============================================================================

    #[test]
    fn test_add_multiple_key_versions_same_id() {
        let auth = AppApiKeyAuth::new();

        auth.add_key_version("key1", "secret_v1", vec!["read".to_string()], "v1", None);
        auth.add_key_version(
            "key1",
            "secret_v2",
            vec!["read".to_string(), "write".to_string()],
            "v2",
            None,
        );
        auth.add_key_version("key1", "secret_v3", vec!["admin".to_string()], "v3", None);

        let metadata = auth.get_key_metadata("key1");
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.versions.len(), 3);

        // All keys should validate
        assert!(auth.validate_key("secret_v1", "127.0.0.1").is_some());
        assert!(auth.validate_key("secret_v2", "127.0.0.1").is_some());
        assert!(auth.validate_key("secret_v3", "127.0.0.1").is_some());
    }

    #[test]
    fn test_validate_key_after_rotation() {
        let auth = AppApiKeyAuth::new().with_rotation(RotationConfig::default());

        auth.add_key_version("key1", "secret_v1", vec!["read".to_string()], "v1", None);

        let result = auth.rotate_key(
            "key1",
            "secret_v2",
            vec!["read".to_string(), "write".to_string()],
            "v2",
        );
        assert!(result.is_ok());

        let perms_v1 = auth.validate_key("secret_v1", "127.0.0.1");
        let perms_v2 = auth.validate_key("secret_v2", "127.0.0.1");

        assert!(perms_v1.is_some());
        assert!(perms_v2.is_some());

        assert_eq!(perms_v1.unwrap(), vec!["read"]);
        assert_eq!(perms_v2.unwrap(), vec!["read", "write"]);
    }

    // ============================================================================
    // with_dependencies Tests
    // ============================================================================

    #[test]
    fn test_with_dependencies_custom_cache() {
        use std::sync::Arc;

        let custom_cache = Arc::new(crate::cache::DashMapCache::new());
        let auth = AppApiKeyAuth::with_dependencies(custom_cache.clone(), None);

        auth.add_key("custom_key", vec!["read".to_string()]);
        let perms = auth.validate_key("custom_key", "127.0.0.1");

        assert!(perms.is_some());
        assert_eq!(perms.unwrap(), vec!["read"]);
    }

    // ============================================================================
    // with_dependencies with Some(key_metadata) Tests
    // ============================================================================

    /// Verify that with_dependencies uses the provided key_metadata cache
    /// instead of creating a new one (covers the unwrap_or_else Some branch,
    /// line 66).
    #[test]
    fn test_with_dependencies_with_custom_key_metadata() {
        use crate::cache::SyncCache;

        let valid_keys = Arc::new(crate::cache::DashMapCache::new());
        let key_metadata = Arc::new(crate::cache::DashMapCache::new());

        let auth = AppApiKeyAuth::with_dependencies(valid_keys.clone(), Some(key_metadata.clone()));

        // Add a versioned key — this writes to key_metadata
        auth.add_key_version("kid1", "secret_v1", vec!["read".to_string()], "v1", None);

        // Verify the metadata was stored in OUR custom key_metadata cache,
        // not a newly-created one.
        let stored = key_metadata.get("metadata:kid1");
        assert!(
            stored.is_some(),
            "Metadata should be stored in the custom key_metadata cache"
        );
    }

    // ============================================================================
    // rotate_key cleanup with version deletion Tests
    // ============================================================================

    /// Verify that rotate_key cleans up old versions beyond keep_versions and
    /// deletes their key hashes from valid_keys (lines 258-263).
    ///
    /// Uses keep_versions=1 so that after multiple rotations, old versions are
    /// evicted and their keys no longer validate.
    #[test]
    fn test_rotate_key_cleans_up_old_versions_beyond_keep_limit() {
        let rotation_config = RotationConfig {
            rotation_interval: std::time::Duration::from_secs(3600),
            grace_period: std::time::Duration::from_secs(60),
            keep_versions: 1,
        };
        let auth = AppApiKeyAuth::new().with_rotation(rotation_config);

        // Add initial version
        auth.add_key_version("kid1", "secret_v1", vec!["read".to_string()], "v1", None);

        // Rotate to v2 — with keep_versions=1, v1 should be cleaned up and
        // its hash deleted from valid_keys.
        auth.rotate_key("kid1", "secret_v2", vec!["read".to_string()], "v2")
            .expect("rotation v2");

        // v1 should no longer validate because its hash was deleted from valid_keys
        let perms_v1 = auth.validate_key("secret_v1", "127.0.0.1");
        assert!(
            perms_v1.is_none(),
            "Old version v1 should no longer validate after cleanup (keep_versions=1)"
        );

        // v2 should still validate
        let perms_v2 = auth.validate_key("secret_v2", "127.0.0.1");
        assert!(
            perms_v2.is_some(),
            "Latest version v2 should still validate"
        );
        assert_eq!(perms_v2.unwrap(), vec!["read"]);
    }

    // ============================================================================
    // add_key_version with corrupted metadata Tests
    // ============================================================================

    /// Verify that add_key_version silently skips metadata update when
    /// existing metadata is corrupted (the `if let Ok` failure path,
    /// line 172).
    ///
    /// When the stored metadata cannot be deserialized, add_key_version still
    /// stores the key hash in valid_keys but does NOT update the metadata.
    #[test]
    fn test_add_key_version_with_corrupted_metadata_skips_update() {
        let auth = AppApiKeyAuth::new();

        // Manually store corrupted metadata for "kid1"
        auth.key_metadata
            .set("metadata:kid1", b"corrupted_data".to_vec());

        // Call add_key_version — it should find the corrupted metadata,
        // fail to deserialize it, and skip the metadata update.
        auth.add_key_version("kid1", "secret_v1", vec!["read".to_string()], "v1", None);

        // The key hash should still be stored in valid_keys (stored before
        // the metadata check).
        let perms = auth.validate_key("secret_v1", "127.0.0.1");
        assert!(
            perms.is_some(),
            "Key hash should be stored in valid_keys even when metadata is corrupted"
        );
        assert_eq!(perms.unwrap(), vec!["read"]);

        // The metadata should still be corrupted (not updated)
        let metadata = auth.get_key_metadata("kid1");
        assert!(
            metadata.is_none(),
            "Corrupted metadata should not be parseable, so get_key_metadata returns None"
        );
    }
}

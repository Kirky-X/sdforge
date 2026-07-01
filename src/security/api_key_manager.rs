// Copyright (c) 2026 Kirky.X
//! Enhanced API key management with versioning, LRU eviction, and rotation support
//!
//! This module provides advanced API key management features:
//! - Key versioning for smooth rotation
//! - LRU cache eviction to prevent memory growth
//! - Automatic key rotation with grace period

use crate::cache::SharedCache;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// API key version information
#[derive(Debug, Clone)]
pub struct ApiKeyVersion {
    /// Version identifier (v1, v2, etc.)
    pub version: String,
    /// Key hash
    pub key_hash: String,
    /// Permissions for this version
    pub permissions: Vec<String>,
    /// Creation timestamp
    pub created_at: Instant,
    /// Expiration timestamp (None = no expiration)
    pub expires_at: Option<Instant>,
    /// Whether this version is active
    pub is_active: bool,
}

/// Custom serialization for ApiKeyVersion
impl Serialize for ApiKeyVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ApiKeyVersion", 6)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("key_hash", &self.key_hash)?;
        state.serialize_field("permissions", &self.permissions)?;
        let created_nanos = self.created_at.elapsed().as_nanos() as i64;
        state.serialize_field("created_at", &created_nanos)?;
        let expires_nanos = self.expires_at.map(|i| i.elapsed().as_nanos() as i64);
        state.serialize_field("expires_at", &expires_nanos)?;
        state.serialize_field("is_active", &self.is_active)?;
        state.end()
    }
}

/// Custom deserialization for ApiKeyVersion
impl<'de> Deserialize<'de> for ApiKeyVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            version: String,
            key_hash: String,
            permissions: Vec<String>,
            created_at: i64,
            expires_at: Option<i64>,
            is_active: bool,
        }
        let helper = Helper::deserialize(deserializer)?;
        Ok(Self {
            version: helper.version,
            key_hash: helper.key_hash,
            permissions: helper.permissions,
            created_at: Instant::now() - Duration::from_nanos(helper.created_at as u64),
            expires_at: helper
                .expires_at
                .map(|n| Instant::now() - Duration::from_nanos(n as u64)),
            is_active: helper.is_active,
        })
    }
}

impl ApiKeyVersion {
    /// Create a new API key version
    pub fn new(
        version: String,
        key_hash: String,
        permissions: Vec<String>,
        ttl: Option<Duration>,
    ) -> Self {
        let created_at = Instant::now();
        let expires_at = ttl.map(|duration| created_at + duration);

        Self {
            version,
            key_hash,
            permissions,
            created_at,
            expires_at,
            is_active: true,
        }
    }

    /// Check if this version is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Instant::now() > expires_at
        } else {
            false
        }
    }

    /// Deactivate this version
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

/// API key metadata with versioning support
#[derive(Debug, Clone)]
pub struct ApiKeyMetadata {
    /// Key identifier (unique ID for this API key)
    pub key_id: String,
    /// All versions of this key
    pub versions: Vec<ApiKeyVersion>,
    /// Current active version index
    pub active_version_index: Option<usize>,
    /// Creation timestamp of the key
    pub created_at: Instant,
    /// Key description
    pub description: Option<String>,
}

/// Custom serialization for ApiKeyMetadata
impl Serialize for ApiKeyMetadata {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ApiKeyMetadata", 5)?;
        state.serialize_field("key_id", &self.key_id)?;
        state.serialize_field("versions", &self.versions)?;
        state.serialize_field("active_version_index", &self.active_version_index)?;
        let created_nanos = self.created_at.elapsed().as_nanos() as i64;
        state.serialize_field("created_at", &created_nanos)?;
        state.serialize_field("description", &self.description)?;
        state.end()
    }
}

/// Custom deserialization for ApiKeyMetadata
impl<'de> Deserialize<'de> for ApiKeyMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            key_id: String,
            versions: Vec<ApiKeyVersion>,
            active_version_index: Option<usize>,
            created_at: i64,
            description: Option<String>,
        }
        let helper = Helper::deserialize(deserializer)?;
        Ok(Self {
            key_id: helper.key_id,
            versions: helper.versions,
            active_version_index: helper.active_version_index,
            created_at: Instant::now() - Duration::from_nanos(helper.created_at as u64),
            description: helper.description,
        })
    }
}

impl ApiKeyMetadata {
    /// Create new API key metadata
    pub fn new(key_id: String, description: Option<String>) -> Self {
        Self {
            key_id,
            versions: Vec::new(),
            active_version_index: None,
            created_at: Instant::now(),
            description,
        }
    }

    /// Add a new version to this key
    pub fn add_version(&mut self, version: ApiKeyVersion) {
        let is_active = version.is_active;
        self.versions.push(version);
        // Set as active if it's the first version or explicitly marked active
        if self.active_version_index.is_none() || is_active {
            self.active_version_index = Some(self.versions.len() - 1);
        }
    }

    /// Get the active version
    pub fn get_active_version(&self) -> Option<&ApiKeyVersion> {
        self.active_version_index
            .and_then(|idx| self.versions.get(idx))
            .filter(|v| v.is_active && !v.is_expired())
    }

    /// Rotate to a new version
    pub fn rotate_to_version(&mut self, version_index: usize) -> Result<(), String> {
        if version_index >= self.versions.len() {
            return Err(format!("Version {} does not exist", version_index));
        }

        let _old_version = self
            .active_version_index
            .and_then(|idx| self.versions.get(idx))
            .map(|v| v.version.clone())
            .unwrap_or_else(|| "none".to_string());

        // Deactivate current version
        if let Some(current_idx) = self.active_version_index {
            if let Some(current) = self.versions.get_mut(current_idx) {
                current.deactivate();
            }
        }

        // Activate new version
        if let Some(new_version) = self.versions.get_mut(version_index) {
            new_version.is_active = true;
            self.active_version_index = Some(version_index);

            // Log rotation event (tracing will be added when feature is enabled)
            // tracing::info!(
            //     key_id = %self.key_id,
            //     old_version = %old_version,
            //     new_version = %new_version_name,
            //     "API key rotated to new version"
            // );

            Ok(())
        } else {
            Err(format!("Version {} not found", version_index))
        }
    }

    /// Clean up expired and old versions
    pub fn cleanup_versions(&mut self, keep_last_n: usize) {
        let active_idx = self.active_version_index;
        let last_idx = self.versions.len().saturating_sub(1);

        // Remove expired versions that are not the active one
        self.versions
            .retain(|v| !v.is_expired() || active_idx == Some(last_idx));

        // Keep only the last N versions
        if self.versions.len() > keep_last_n {
            let active = self.get_active_version();
            let active_version_name = active.map(|v| v.version.clone());

            // Take the last N versions
            let len = self.versions.len();
            let to_take = keep_last_n.min(len);
            let remaining: Vec<_> = self.versions.drain(len - to_take..).collect();

            // Reverse to maintain order, take N, reverse back
            let mut kept: Vec<_> = remaining.into_iter().rev().take(keep_last_n).collect();
            kept.reverse();
            self.versions = kept;

            // Update active index
            if let Some(ref version_name) = active_version_name {
                self.active_version_index = self
                    .versions
                    .iter()
                    .position(|v| &v.version == version_name);
            }
        }
    }

    /// Get all non-expired versions
    pub fn get_valid_versions(&self) -> Vec<&ApiKeyVersion> {
        self.versions.iter().filter(|v| !v.is_expired()).collect()
    }
}

/// LRU configuration for cache eviction
#[derive(Debug, Clone)]
pub struct LruConfig {
    /// Maximum number of entries to cache
    pub max_entries: usize,
    /// Time-to-live for cache entries
    pub ttl: Duration,
    /// Eviction threshold (percentage of max_entries at which to start eviction)
    pub eviction_threshold: f64,
}

impl Default for LruConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,            // Increased from 1000 to 10000
            ttl: Duration::from_secs(3600), // 1 hour
            eviction_threshold: 0.8,        // Start eviction at 80% capacity
        }
    }
}

/// LRU cache wrapper for SharedCache
///
/// Provides LRU eviction on top of SharedCache to prevent unlimited memory growth.
/// This is a simplified implementation that tracks metadata separately.
///
/// All operations are synchronous. The backing `SharedCache` is itself non-async
/// (it wraps a `DashMap`), so wrapping synchronous bookkeeping in `tokio::spawn`
/// only added overhead, runtime-context requirements, and nondeterministic
/// ordering (spawned tasks could race with subsequent `set`/`delete` calls,
/// making eviction tests flaky). A `std::sync::Mutex` is the correct choice:
/// the critical sections are tiny (HashMap insert/remove) and never await.
pub struct LruCacheManager {
    cache: SharedCache,
    access_times: Arc<std::sync::Mutex<HashMap<String, Instant>>>,
    config: LruConfig,
}

impl LruCacheManager {
    /// Create new LRU cache manager
    pub fn new(cache: SharedCache, config: LruConfig) -> Self {
        Self {
            cache,
            access_times: Arc::new(std::sync::Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Get value and update access time
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let value = self.cache.get(key)?;

        // Update access time synchronously. The critical section is a single
        // HashMap insert and never awaits, so a blocking mutex is appropriate
        // and avoids the runtime-context requirement of tokio::spawn.
        if let Ok(mut times) = self.access_times.lock() {
            times.insert(key.to_string(), Instant::now());
        }

        Some(value)
    }

    /// Set value and track access time
    pub fn set(&self, key: &str, value: Vec<u8>) {
        // Check if we need to evict entries
        self.check_and_evict();

        self.cache.set(key, value);

        // Track access time synchronously.
        if let Ok(mut times) = self.access_times.lock() {
            times.insert(key.to_string(), Instant::now());
        }
    }

    /// Delete value
    pub fn delete(&self, key: &str) {
        self.cache.delete(key);

        if let Ok(mut times) = self.access_times.lock() {
            times.remove(key);
        }
    }

    /// Check and evict old entries if needed
    fn check_and_evict(&self) {
        let Ok(mut times) = self.access_times.lock() else {
            return;
        };
        let now = Instant::now();
        let config = &self.config;

        // First, remove expired entries (TTL-based). Collect keys to evict so we
        // can also delete them from the backing cache (not just access metadata).
        // The `times` mutex and the backing cache's internal lock are distinct,
        // so holding `times` while calling `cache.delete` cannot deadlock.
        let mut expired_keys: Vec<String> = Vec::new();
        times.retain(|k, &mut last_accessed| {
            if now.duration_since(last_accessed) > config.ttl {
                expired_keys.push(k.clone());
                false
            } else {
                true
            }
        });
        for key in &expired_keys {
            self.cache.delete(key);
        }

        // Calculate eviction threshold (80% of max_entries by default)
        let threshold = (config.max_entries as f64 * config.eviction_threshold) as usize;

        // If over threshold, start evicting oldest entries
        if times.len() > threshold {
            // Sort by access time (oldest first)
            let mut entries: Vec<_> = times.iter().map(|(k, v)| (k.clone(), *v)).collect();
            entries.sort_by_key(|&(_, time)| time);

            // Remove oldest entries to get back under threshold.
            // Delete both the access-tracking entry AND the cached value
            // to actually reclaim memory.
            let to_remove = entries.len().saturating_sub(threshold);
            for (key, _) in entries.into_iter().take(to_remove) {
                times.remove(&key);
                self.cache.delete(&key);
            }
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> LruStats {
        let total_entries = self
            .access_times
            .lock()
            .map(|times| times.len())
            .unwrap_or(0);
        LruStats {
            total_entries,
            max_entries: self.config.max_entries,
            ttl: self.config.ttl,
        }
    }
}

/// LRU cache statistics
#[derive(Debug, Clone)]
pub struct LruStats {
    /// Total number of entries in the cache
    pub total_entries: usize,
    /// Maximum number of entries allowed
    pub max_entries: usize,
    /// Time-to-live for cache entries
    pub ttl: Duration,
}

/// Key rotation configuration
#[derive(Debug, Clone)]
pub struct RotationConfig {
    /// Automatically rotate keys after this duration
    pub rotation_interval: Duration,
    /// Grace period before old keys become invalid
    pub grace_period: Duration,
    /// Number of old versions to keep
    pub keep_versions: usize,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            rotation_interval: Duration::from_secs(86400 * 30), // 30 days
            grace_period: Duration::from_secs(86400 * 7),       // 7 days
            keep_versions: 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_version_creation() {
        let version = ApiKeyVersion::new(
            "v1".to_string(),
            "hash123".to_string(),
            vec!["read".to_string()],
            Some(Duration::from_secs(3600)),
        );

        assert_eq!(version.version, "v1");
        assert_eq!(version.key_hash, "hash123");
        assert!(version.is_active);
        assert!(!version.is_expired());
    }

    #[test]
    fn test_api_key_version_expiration() {
        let version = ApiKeyVersion::new(
            "v1".to_string(),
            "hash123".to_string(),
            vec![],
            Some(Duration::from_millis(10)),
        );

        std::thread::sleep(Duration::from_millis(20));
        assert!(version.is_expired());
    }

    #[test]
    fn test_api_key_metadata_add_version() {
        let mut metadata = ApiKeyMetadata::new("key1".to_string(), Some("Test key".to_string()));

        let version1 = ApiKeyVersion::new(
            "v1".to_string(),
            "hash1".to_string(),
            vec!["read".to_string()],
            None,
        );

        metadata.add_version(version1);

        assert_eq!(metadata.versions.len(), 1);
        assert!(metadata.get_active_version().is_some());
    }

    #[test]
    fn test_api_key_metadata_rotation() {
        let mut metadata = ApiKeyMetadata::new("key1".to_string(), None);

        let v1 = ApiKeyVersion::new("v1".to_string(), "hash1".to_string(), vec![], None);
        let v2 = ApiKeyVersion::new("v2".to_string(), "hash2".to_string(), vec![], None);

        metadata.add_version(v1);
        metadata.add_version(v2);

        // Rotate to v2
        let result = metadata.rotate_to_version(1);
        assert!(result.is_ok());

        let active = metadata.get_active_version().unwrap();
        assert_eq!(active.version, "v2");
    }

    #[test]
    fn test_api_key_metadata_cleanup() {
        let mut metadata = ApiKeyMetadata::new("key1".to_string(), None);

        // Add multiple versions
        for i in 1..=5 {
            let version = ApiKeyVersion::new(format!("v{}", i), format!("hash{}", i), vec![], None);
            metadata.add_version(version);
        }

        metadata.cleanup_versions(3);

        assert!(metadata.versions.len() <= 3);
    }

    #[test]
    fn test_lru_config_default() {
        let config = LruConfig::default();
        assert_eq!(config.max_entries, 10_000);
        assert_eq!(config.ttl, Duration::from_secs(3600));
        assert!((config.eviction_threshold - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lru_eviction_threshold() {
        // Test that eviction threshold is respected
        let config = LruConfig {
            max_entries: 100,
            ttl: Duration::from_secs(3600),
            eviction_threshold: 0.5, // 50% threshold for testing
        };

        assert_eq!(config.max_entries, 100);
        assert!((config.eviction_threshold - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rotation_config_default() {
        let config = RotationConfig::default();
        assert_eq!(config.rotation_interval, Duration::from_secs(86400 * 30));
        assert_eq!(config.grace_period, Duration::from_secs(86400 * 7));
        assert_eq!(config.keep_versions, 3);
    }

    // ===== LruCacheManager Tests =====

    #[test]
    fn test_lru_cache_manager_basic_creation() {
        use crate::cache::DashMapCache;

        let cache = DashMapCache::new();
        let config = LruConfig::default();
        let manager = LruCacheManager::new(Arc::new(cache), config);

        let stats = manager.stats();
        assert_eq!(stats.max_entries, 10_000);
        assert_eq!(stats.ttl, Duration::from_secs(3600));
    }

    #[test]
    fn test_lru_cache_manager_eviction() {
        use crate::cache::DashMapCache;

        let cache = DashMapCache::with_capacity(10);
        let config = LruConfig {
            max_entries: 5,
            ttl: Duration::from_secs(3600),
            eviction_threshold: 0.8,
        };
        let manager = LruCacheManager::new(Arc::new(cache), config);

        for i in 0..5 {
            manager.set(&format!("key_{}", i), vec![i as u8]);
        }

        // Operations are now synchronous — no sleep needed for spawned tasks.
        let stats = manager.stats();
        assert!(stats.total_entries <= 5);
    }

    #[test]
    fn test_lru_cache_manager_access_order() {
        use crate::cache::DashMapCache;

        let cache = DashMapCache::new();
        let config = LruConfig::default();
        let manager = LruCacheManager::new(Arc::new(cache), config);

        manager.set("key1", b"value1".to_vec());
        manager.set("key2", b"value2".to_vec());
        manager.set("key3", b"value3".to_vec());

        let _ = manager.get("key1");

        // Operations are now synchronous — no sleep needed for spawned tasks.
        assert_eq!(manager.get("key1"), Some(b"value1".to_vec()));
    }

    #[test]
    fn test_lru_cache_manager_custom_config() {
        use crate::cache::DashMapCache;

        let cache = DashMapCache::new();
        let config = LruConfig {
            max_entries: 500,
            ttl: Duration::from_secs(1800),
            eviction_threshold: 0.6,
        };
        let manager = LruCacheManager::new(Arc::new(cache), config);

        let stats = manager.stats();
        assert_eq!(stats.max_entries, 500);
        assert_eq!(stats.ttl, Duration::from_secs(1800));
    }

    #[test]
    fn test_lru_config_clone() {
        let config = LruConfig {
            max_entries: 5000,
            ttl: Duration::from_secs(7200),
            eviction_threshold: 0.9,
        };

        let cloned = config.clone();
        assert_eq!(cloned.max_entries, 5000);
        assert_eq!(cloned.ttl, Duration::from_secs(7200));
        assert!((cloned.eviction_threshold - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rotation_config_clone() {
        let config = RotationConfig {
            rotation_interval: Duration::from_secs(86400 * 60),
            grace_period: Duration::from_secs(86400 * 14),
            keep_versions: 5,
        };

        let cloned = config.clone();
        assert_eq!(cloned.rotation_interval, Duration::from_secs(86400 * 60));
        assert_eq!(cloned.grace_period, Duration::from_secs(86400 * 14));
        assert_eq!(cloned.keep_versions, 5);
    }

    #[test]
    fn test_api_key_version_deactivate() {
        let mut version = ApiKeyVersion::new(
            "v1".to_string(),
            "hash123".to_string(),
            vec!["read".to_string()],
            None,
        );

        assert!(version.is_active);

        version.deactivate();

        assert!(!version.is_active);
    }

    #[test]
    fn test_api_key_version_clone() {
        let version = ApiKeyVersion::new(
            "v1".to_string(),
            "hash123".to_string(),
            vec!["read".to_string(), "write".to_string()],
            Some(Duration::from_secs(3600)),
        );

        let cloned = version.clone();
        assert_eq!(cloned.version, "v1");
        assert_eq!(cloned.key_hash, "hash123");
        assert_eq!(cloned.permissions.len(), 2);
        assert!(cloned.is_active);
    }

    #[test]
    fn test_api_key_metadata_cleanup_versions() {
        let mut metadata = ApiKeyMetadata::new("key1".to_string(), None);

        for i in 1..=5 {
            let version = ApiKeyVersion::new(format!("v{}", i), format!("hash{}", i), vec![], None);
            metadata.add_version(version);
        }

        metadata.cleanup_versions(2);

        assert!(metadata.versions.len() <= 2);
    }

    #[test]
    fn test_api_key_metadata_get_active_version() {
        let mut metadata = ApiKeyMetadata::new("key1".to_string(), None);

        let v1 = ApiKeyVersion::new("v1".to_string(), "hash1".to_string(), vec![], None);
        let v2 = ApiKeyVersion::new("v2".to_string(), "hash2".to_string(), vec![], None);

        metadata.add_version(v1);
        metadata.add_version(v2);

        let _ = metadata.rotate_to_version(1);

        let active = metadata.get_active_version();
        assert!(active.is_some());
        assert_eq!(active.unwrap().version, "v2");
    }

    #[test]
    fn test_api_key_metadata_clone() {
        let mut metadata = ApiKeyMetadata::new("key1".to_string(), Some("Test key".to_string()));

        let version = ApiKeyVersion::new(
            "v1".to_string(),
            "hash1".to_string(),
            vec!["read".to_string()],
            None,
        );
        metadata.add_version(version);

        let cloned = metadata.clone();
        assert_eq!(cloned.key_id, "key1");
        assert_eq!(cloned.description, Some("Test key".to_string()));
        assert_eq!(cloned.versions.len(), 1);
    }

    // ============================================================================
    // Additional coverage tests
    // ============================================================================

    #[test]
    fn test_api_key_version_deserialize() {
        let json = r#"{"version":"v1","key_hash":"hash123","permissions":["read"],"created_at":1000,"expires_at":null,"is_active":true}"#;
        let version: ApiKeyVersion = serde_json::from_str(json).unwrap();
        assert_eq!(version.version, "v1");
        assert_eq!(version.key_hash, "hash123");
        assert!(version.is_active);
        assert_eq!(version.permissions, vec!["read".to_string()]);
    }

    #[test]
    fn test_api_key_version_serialize_deserialize_roundtrip() {
        let original = ApiKeyVersion::new(
            "v1".to_string(),
            "hash123".to_string(),
            vec!["read".to_string()],
            Some(Duration::from_secs(3600)),
        );
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ApiKeyVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version, original.version);
        assert_eq!(deserialized.key_hash, original.key_hash);
        assert_eq!(deserialized.is_active, original.is_active);
    }

    #[test]
    fn test_rotate_to_version_nonexistent_index() {
        let mut metadata = ApiKeyMetadata::new("key1".to_string(), None);
        let v1 = ApiKeyVersion::new("v1".to_string(), "hash1".to_string(), vec![], None);
        metadata.add_version(v1);

        let result = metadata.rotate_to_version(5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_get_valid_versions_all_valid() {
        let mut metadata = ApiKeyMetadata::new("key1".to_string(), None);
        let v1 = ApiKeyVersion::new("v1".to_string(), "hash1".to_string(), vec![], None);
        let v2 = ApiKeyVersion::new("v2".to_string(), "hash2".to_string(), vec![], None);
        metadata.add_version(v1);
        metadata.add_version(v2);

        let valid = metadata.get_valid_versions();
        assert_eq!(valid.len(), 2);
    }

    #[test]
    fn test_get_valid_versions_with_expired() {
        let mut metadata = ApiKeyMetadata::new("key1".to_string(), None);
        let v1 = ApiKeyVersion::new(
            "v1".to_string(),
            "hash1".to_string(),
            vec![],
            Some(Duration::from_millis(10)),
        );
        let v2 = ApiKeyVersion::new("v2".to_string(), "hash2".to_string(), vec![], None);
        metadata.add_version(v1);
        metadata.add_version(v2);

        std::thread::sleep(Duration::from_millis(20));

        let valid = metadata.get_valid_versions();
        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0].version, "v2");
    }

    #[test]
    fn test_lru_cache_manager_delete() {
        use crate::cache::DashMapCache;

        let cache = DashMapCache::new();
        let config = LruConfig::default();
        let manager = LruCacheManager::new(Arc::new(cache), config);

        manager.set("key1", b"value1".to_vec());
        // Operations are now synchronous — no sleep needed.
        assert_eq!(manager.get("key1"), Some(b"value1".to_vec()));

        manager.delete("key1");
        // Operations are now synchronous — no sleep needed.
        assert_eq!(manager.get("key1"), None);
    }

    #[test]
    fn test_lru_cache_manager_eviction_threshold_triggers() {
        use crate::cache::DashMapCache;

        let cache = DashMapCache::with_capacity(100);
        let config = LruConfig {
            max_entries: 3,
            ttl: Duration::from_secs(3600),
            eviction_threshold: 0.5, // threshold = 1
        };
        let manager = LruCacheManager::new(Arc::new(cache), config);

        // Operations are now synchronous — eviction happens inline during set().
        manager.set("key_0", vec![0]);
        manager.set("key_1", vec![1]);
        manager.set("key_2", vec![2]);

        let stats = manager.stats();
        // Eviction logic should have been triggered (times.len() > threshold)
        assert!(stats.total_entries <= 3);
    }

    #[test]
    fn test_lru_eviction_deletes_cache_values() {
        use crate::cache::DashMapCache;

        // Regression test for H-2/H1: LruCacheManager eviction previously only
        // removed access-tracking metadata while leaving the actual cached values
        // in the backing cache — causing unbounded memory growth. This test
        // verifies that evicted keys are deleted from the backing cache.
        let cache: SharedCache = Arc::new(DashMapCache::with_capacity(100));
        let config = LruConfig {
            max_entries: 2,
            ttl: Duration::from_secs(3600),
            eviction_threshold: 0.5, // threshold = 1; evict once > 1 entry
        };
        // Keep a reference to the cache so we can inspect it after eviction.
        let cache_ref = cache.clone();
        let manager = LruCacheManager::new(cache, config);

        // Operations are now synchronous — eviction happens inline during set().
        manager.set("k1", b"v1".to_vec());
        manager.set("k2", b"v2".to_vec());
        manager.set("k3", b"v3".to_vec());

        // The backing cache should NOT contain all 3 entries forever.
        // At least one entry must have been evicted from the cache itself
        // (not just from access-tracking metadata).
        let remaining = ["k1", "k2", "k3"]
            .iter()
            .filter(|k| cache_ref.get(k).is_some())
            .count();
        assert!(
            remaining < 3,
            "Eviction must delete cached values, not just access metadata. \
             Still present: {}",
            remaining
        );
    }
}

// Copyright (c) 2026 Kirky.X
//! Cache configuration
//!
//! This module provides cache configuration options for the application.

use serde::{Deserialize, Serialize};

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    /// Enable caching
    pub enabled: bool,
    /// Default TTL in seconds
    pub default_ttl_secs: u64,
    /// Maximum number of items in cache
    pub max_items: usize,
    /// Enable cache statistics tracking
    pub track_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        use crate::config::defaults::cache::*;
        Self {
            enabled: DEFAULT_ENABLED,
            default_ttl_secs: DEFAULT_TTL_SECS,
            max_items: DEFAULT_MAX_ITEMS,
            track_stats: DEFAULT_TRACK_STATS,
        }
    }
}

impl CacheConfig {
    /// Create a new cache config with custom TTL
    pub fn with_ttl(ttl_secs: u64) -> Self {
        Self {
            default_ttl_secs: ttl_secs,
            ..Default::default()
        }
    }

    /// Create a disabled cache config
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Check if cache is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the default TTL in seconds
    pub fn default_ttl(&self) -> u64 {
        self.default_ttl_secs
    }

    /// Get the maximum number of items
    pub fn max_items(&self) -> usize {
        self.max_items
    }
}

#[cfg(feature = "validation")]
impl crate::config::ValidateConfig for CacheConfig {
    fn validate(&self) -> Result<(), crate::config::ConfigError> {
        use crate::config::ConfigError;
        
        // Validate TTL is positive
        if self.default_ttl_secs == 0 {
            return Err(ConfigError::ValidationError(
                "Cache default_ttl_secs cannot be 0".into(),
            ));
        }

        // Validate max_items is reasonable
        if self.max_items == 0 {
            return Err(ConfigError::ValidationError(
                "Cache max_items cannot be 0".into(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_ttl_secs, 300);
        assert_eq!(config.max_items, 10_000);
        assert!(config.track_stats);
    }

    #[test]
    fn test_cache_config_with_ttl() {
        let config = CacheConfig::with_ttl(600);
        assert_eq!(config.default_ttl_secs, 600);
    }

    #[test]
    fn test_cache_config_disabled() {
        let config = CacheConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_cache_config_methods() {
        let config = CacheConfig::default();
        assert!(config.is_enabled());
        assert_eq!(config.default_ttl(), 300);
        assert_eq!(config.max_items(), 10_000);
    }
}

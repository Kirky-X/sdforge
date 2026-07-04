// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Timeout configuration module
//!
//! This module provides timeout-related configuration types.

use crate::config::defaults;
use serde::{Deserialize, Serialize};

/// Timeout configuration for different routes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeoutConfig {
    /// Default request timeout in seconds
    #[serde(default = "default_timeout")]
    pub default_timeout_secs: u64,
    /// Route-specific timeouts
    #[serde(default)]
    pub route_timeouts: std::collections::HashMap<String, u64>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout_secs: default_timeout(),
            route_timeouts: default_route_timeouts(),
        }
    }
}

fn default_timeout() -> u64 {
    defaults::timeout::DEFAULT_TIMEOUT_SECS
}

fn default_route_timeouts() -> std::collections::HashMap<String, u64> {
    let mut route_timeouts = std::collections::HashMap::new();
    route_timeouts.insert(
        "/api/upload".to_string(),
        defaults::timeout::UPLOAD_TIMEOUT_SECS,
    );
    route_timeouts.insert(
        "/api/export".to_string(),
        defaults::timeout::EXPORT_TIMEOUT_SECS,
    );
    route_timeouts
}

impl TimeoutConfig {
    /// Get timeout for a specific route
    pub fn get_timeout(&self, path: &str) -> u64 {
        self.route_timeouts
            .get(path)
            .copied()
            .unwrap_or(self.default_timeout_secs)
    }

    /// Validate timeout configuration
    pub fn validate(&self) -> Result<(), crate::config::ConfigError> {
        if self.default_timeout_secs == 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "timeout.default_timeout_secs must be greater than 0".into(),
            ));
        }

        // Check for unreasonably long timeouts
        if self.default_timeout_secs > 3600 {
            return Err(crate::config::ConfigError::ValidationError(
                "timeout.default_timeout_secs should not exceed 3600 seconds (1 hour)".into(),
            ));
        }

        // Validate route-specific timeouts
        for (route, timeout) in &self.route_timeouts {
            if *timeout == 0 {
                return Err(crate::config::ConfigError::ValidationError(format!(
                    "timeout for route '{}' cannot be 0",
                    route
                )));
            }
            if *timeout > 3600 {
                return Err(crate::config::ConfigError::ValidationError(format!(
                    "timeout for route '{}' should not exceed 3600 seconds (1 hour)",
                    route
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test TimeoutConfig defaults and get_timeout
    #[test]
    fn test_timeout_config() {
        let config = TimeoutConfig::default();
        assert_eq!(config.default_timeout_secs, 30);
        assert_eq!(config.get_timeout("/api/upload"), 300);
        assert_eq!(config.get_timeout("/api/export"), 120);
        assert_eq!(config.get_timeout("/unknown"), 30); // Uses default
    }

    #[test]
    fn test_timeout_config_custom_route_timeouts() {
        let mut route_timeouts = std::collections::HashMap::new();
        route_timeouts.insert("/api/custom".to_string(), 600);
        route_timeouts.insert("/api/long".to_string(), 900);
        let config = TimeoutConfig {
            default_timeout_secs: 45,
            route_timeouts,
        };
        assert_eq!(config.get_timeout("/api/custom"), 600);
        assert_eq!(config.get_timeout("/api/long"), 900);
        assert_eq!(config.get_timeout("/api/other"), 45);
    }

    #[test]
    fn test_timeout_config_serialization() {
        let config = TimeoutConfig {
            default_timeout_secs: 60,
            route_timeouts: std::collections::HashMap::new(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: TimeoutConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.default_timeout_secs, 60);
    }

    #[test]
    fn test_timeout_config_validate_zero_default() {
        let config = TimeoutConfig {
            default_timeout_secs: 0,
            route_timeouts: std::collections::HashMap::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_timeout_config_validate_excessive_default() {
        let config = TimeoutConfig {
            default_timeout_secs: 4000, // > 3600
            route_timeouts: std::collections::HashMap::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_timeout_config_validate_zero_route_timeout() {
        let mut route_timeouts = std::collections::HashMap::new();
        route_timeouts.insert("/api/test".to_string(), 0);
        let config = TimeoutConfig {
            default_timeout_secs: 30,
            route_timeouts,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_timeout_config_validate_excessive_route_timeout() {
        let mut route_timeouts = std::collections::HashMap::new();
        route_timeouts.insert("/api/test".to_string(), 4000);
        let config = TimeoutConfig {
            default_timeout_secs: 30,
            route_timeouts,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_timeout_config_validate_valid() {
        let config = TimeoutConfig {
            default_timeout_secs: 30,
            route_timeouts: std::collections::HashMap::new(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_default_timeout_function() {
        assert_eq!(default_timeout(), 30);
    }

    #[test]
    fn test_default_route_timeouts_function() {
        let route_timeouts = default_route_timeouts();
        assert_eq!(route_timeouts.get("/api/upload"), Some(&300));
        assert_eq!(route_timeouts.get("/api/export"), Some(&120));
    }
}

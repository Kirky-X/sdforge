// Copyright (c) 2026 Kirky.X
//! API and utility configuration module
//!
//! This module provides API-related configuration types and utility configs.

use crate::config::Config;
use serde::{Deserialize, Serialize};

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ApiConfig {
    /// API prefix
    pub prefix: String,
    /// Default version
    pub default_version: String,
}

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct TracingConfig {
    /// Tracing enabled
    pub enabled: bool,
}

/// Environment helper
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct EnvHelper {
    /// Environment name
    pub environment: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn test_tracing_config_enabled() {
        let config = TracingConfig { enabled: true };
        assert!(config.enabled);
    }

    #[test]
    fn test_tracing_config_serialization() {
        let config = TracingConfig { enabled: true };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: TracingConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.enabled);
    }

    #[test]
    fn test_env_helper_default() {
        let config = EnvHelper::default();
        assert!(config.environment.is_empty());
    }

    #[test]
    fn test_env_helper_custom() {
        let config = EnvHelper {
            environment: "production".to_string(),
        };
        assert_eq!(config.environment, "production");
    }

    #[test]
    fn test_env_helper_serialization() {
        let config = EnvHelper {
            environment: "staging".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: EnvHelper = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.environment, "staging");
    }

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert!(config.prefix.is_empty());
        assert!(config.default_version.is_empty());
    }

    #[test]
    fn test_api_config_custom() {
        let config = ApiConfig {
            prefix: "/api".to_string(),
            default_version: "v1".to_string(),
        };
        assert_eq!(config.prefix, "/api");
        assert_eq!(config.default_version, "v1");
    }

    #[test]
    fn test_api_config_serialization() {
        let config = ApiConfig {
            prefix: "/v2".to_string(),
            default_version: "v2".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ApiConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.prefix, "/v2");
        assert_eq!(deserialized.default_version, "v2");
    }
}

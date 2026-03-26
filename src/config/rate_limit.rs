// Copyright (c) 2026 Kirky.X
//! Rate limiting configuration module
//!
//! This module provides rate limiting-related configuration types.

use crate::config::{defaults, Config};
use serde::{Deserialize, Serialize};

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct RateLimitConfigFile {
    /// Requests per window
    pub requests: u32,
    /// Window duration in seconds
    pub window_seconds: u64,
}

impl RateLimitConfigFile {
    /// Validate rate limit configuration
    pub fn validate(&self) -> Result<(), crate::config::ConfigError> {
        if self.requests == 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "rate_limit.requests must be greater than 0 when rate limiting is enabled".into(),
            ));
        }
        if self.window_seconds == 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "rate_limit.window_seconds must be greater than 0".into(),
            ));
        }
        if self.window_seconds > defaults::rate_limit::MAX_WINDOW_SECS {
            return Err(crate::config::ConfigError::ValidationError(format!(
                "rate_limit.window_seconds exceeds maximum allowed value of {} seconds",
                defaults::rate_limit::MAX_WINDOW_SECS
            )));
        }
        Ok(())
    }
}

/// Rate limit endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct RateLimitEndpointConfig {
    /// Endpoint path
    pub path: String,
    /// Rate limit for this endpoint
    pub config: RateLimitConfigFile,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test RateLimitConfigFile
    #[test]
    fn test_rate_limit_config() {
        let json = r#"{"requests": 100, "window_seconds": 60}"#;
        let config: RateLimitConfigFile = serde_json::from_str(json).unwrap();
        assert_eq!(config.requests, 100);
        assert_eq!(config.window_seconds, 60);
    }

    #[test]
    fn test_rate_limit_config_serialization() {
        let config = RateLimitConfigFile {
            requests: 500,
            window_seconds: 120,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RateLimitConfigFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.requests, 500);
        assert_eq!(deserialized.window_seconds, 120);
    }

    #[test]
    fn test_rate_limit_config_zero_requests() {
        let config = RateLimitConfigFile {
            requests: 0,
            window_seconds: 60,
        };
        assert_eq!(config.requests, 0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rate_limit_config_zero_window() {
        let config = RateLimitConfigFile {
            requests: 100,
            window_seconds: 0,
        };
        assert_eq!(config.window_seconds, 0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rate_limit_config_validate_valid() {
        let config = RateLimitConfigFile {
            requests: 100,
            window_seconds: 60,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_rate_limit_config_validate_excessive_window() {
        let config = RateLimitConfigFile {
            requests: 100,
            window_seconds: defaults::rate_limit::MAX_WINDOW_SECS + 1,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rate_limit_endpoint_config() {
        let config = RateLimitEndpointConfig {
            path: "/api/heavy".to_string(),
            config: RateLimitConfigFile {
                requests: 10,
                window_seconds: 60,
            },
        };
        assert_eq!(config.path, "/api/heavy");
        assert_eq!(config.config.requests, 10);
    }

    #[test]
    fn test_rate_limit_endpoint_config_serialization() {
        let config = RateLimitEndpointConfig {
            path: "/api/upload".to_string(),
            config: RateLimitConfigFile {
                requests: 5,
                window_seconds: 300,
            },
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RateLimitEndpointConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, "/api/upload");
        assert_eq!(deserialized.config.requests, 5);
        assert_eq!(deserialized.config.window_seconds, 300);
    }
}

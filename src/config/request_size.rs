// Copyright (c) 2026 Kirky.X
//! Request size configuration module
//!
//! This module provides request size limit configuration types.

use crate::config::{defaults, Config};
use serde::{Deserialize, Serialize};

/// Request size configuration for different content types
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[serde(default)]
pub struct RequestSizeConfig {
    /// Maximum JSON request body size (default 1MB)
    #[serde(default = "default_max_json_size")]
    #[config(default = default_max_json_size())]
    pub max_json_size: usize,
    /// Maximum file upload size (default 100MB)
    #[serde(default = "default_max_file_size")]
    #[config(default = default_max_file_size())]
    pub max_file_size: usize,
    /// Maximum form data size (default 10MB)
    #[serde(default = "default_max_form_size")]
    #[config(default = default_max_form_size())]
    pub max_form_size: usize,
}

fn default_max_json_size() -> usize {
    defaults::request_size::MAX_JSON_SIZE
}

fn default_max_file_size() -> usize {
    defaults::request_size::MAX_FILE_SIZE
}

fn default_max_form_size() -> usize {
    defaults::request_size::MAX_FORM_SIZE
}

impl RequestSizeConfig {
    /// Validate request size configuration
    pub fn validate(&self) -> Result<(), crate::config::ConfigError> {
        if self.max_json_size == 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "request_size.max_json_size must be greater than 0".into(),
            ));
        }
        if self.max_file_size == 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "request_size.max_file_size must be greater than 0".into(),
            ));
        }
        if self.max_form_size == 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "request_size.max_form_size must be greater than 0".into(),
            ));
        }

        // Reasonable upper bounds (10GB)
        const MAX_REASONABLE_SIZE: usize = 10 * 1024 * 1024 * 1024;

        if self.max_json_size > MAX_REASONABLE_SIZE {
            return Err(crate::config::ConfigError::ValidationError(format!(
                "request_size.max_json_size exceeds reasonable maximum of {} bytes",
                MAX_REASONABLE_SIZE
            )));
        }
        if self.max_file_size > MAX_REASONABLE_SIZE {
            return Err(crate::config::ConfigError::ValidationError(format!(
                "request_size.max_file_size exceeds reasonable maximum of {} bytes",
                MAX_REASONABLE_SIZE
            )));
        }
        if self.max_form_size > MAX_REASONABLE_SIZE {
            return Err(crate::config::ConfigError::ValidationError(format!(
                "request_size.max_form_size exceeds reasonable maximum of {} bytes",
                MAX_REASONABLE_SIZE
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test RequestSizeConfig defaults
    #[test]
    fn test_request_size_config_defaults() {
        let config = RequestSizeConfig::default();
        assert_eq!(config.max_json_size, 1024 * 1024);
        assert_eq!(config.max_file_size, 100 * 1024 * 1024);
        assert_eq!(config.max_form_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_request_size_config_serialization() {
        let config = RequestSizeConfig {
            max_json_size: 2 * 1024 * 1024,
            max_file_size: 200 * 1024 * 1024,
            max_form_size: 20 * 1024 * 1024,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RequestSizeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_json_size, 2 * 1024 * 1024);
        assert_eq!(deserialized.max_file_size, 200 * 1024 * 1024);
        assert_eq!(deserialized.max_form_size, 20 * 1024 * 1024);
    }

    #[test]
    fn test_request_size_config_partial_json() {
        let json = r#"{"max_json_size": 524288}"#;
        let config: RequestSizeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.max_json_size, 524288);
        assert_eq!(config.max_file_size, default_max_file_size());
        assert_eq!(config.max_form_size, default_max_form_size());
    }

    #[test]
    fn test_request_size_config_validate_zero_json() {
        let config = RequestSizeConfig {
            max_json_size: 0,
            max_file_size: 1024,
            max_form_size: 1024,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_request_size_config_validate_zero_file() {
        let config = RequestSizeConfig {
            max_json_size: 1024,
            max_file_size: 0,
            max_form_size: 1024,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_request_size_config_validate_zero_form() {
        let config = RequestSizeConfig {
            max_json_size: 1024,
            max_file_size: 1024,
            max_form_size: 0,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_request_size_config_validate_excessive_size() {
        let config = RequestSizeConfig {
            max_json_size: 11 * 1024 * 1024 * 1024, // > 10GB
            max_file_size: 1024,
            max_form_size: 1024,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_request_size_config_validate_valid() {
        let config = RequestSizeConfig {
            max_json_size: 1024 * 1024,
            max_file_size: 100 * 1024 * 1024,
            max_form_size: 10 * 1024 * 1024,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_default_max_json_size_function() {
        assert_eq!(default_max_json_size(), 1024 * 1024);
    }

    #[test]
    fn test_default_max_file_size_function() {
        assert_eq!(default_max_file_size(), 100 * 1024 * 1024);
    }

    #[test]
    fn test_default_max_form_size_function() {
        assert_eq!(default_max_form_size(), 10 * 1024 * 1024);
    }
}

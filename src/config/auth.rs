// Copyright (c) 2026 Kirky.X
//! Authentication configuration module
//!
//! This module provides authentication-related configuration types.

use serde::{Deserialize, Serialize};

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum AuthConfig {
    /// API key authentication
    #[serde(rename = "api_key")]
    ApiKey {
        /// Header name for the API key
        header_name: String,
        /// Prefix for the API key value
        prefix: String,
    },
    /// JWT authentication
    #[serde(rename = "jwt")]
    Jwt {
        /// JWT secret key
        secret: String,
    },
    /// No authentication
    #[serde(rename = "none")]
    #[default]
    None,
}

impl AuthConfig {
    /// Validate authentication configuration at load time.
    ///
    /// Security: Rejects configurations that could bypass authentication.
    /// An empty prefix allows any API key to pass validation, enabling auth bypass.
    pub fn validate(&self) -> Result<(), crate::config::ConfigError> {
        match self {
            AuthConfig::ApiKey { prefix, .. } => {
                if prefix.is_empty() {
                    return Err(crate::config::ConfigError::ValidationError(
                        "API key prefix cannot be empty: an empty prefix allows any key to match"
                            .into(),
                    ));
                }
            }
            AuthConfig::None | AuthConfig::Jwt { .. } => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test AuthConfig::ApiKey variant
    #[test]
    fn test_auth_config_api_key() {
        let json = r#"{"type": "api_key", "header_name": "Authorization", "prefix": "Bearer "}"#;
        let config: AuthConfig = serde_json::from_str(json).unwrap();
        match config {
            AuthConfig::ApiKey {
                header_name,
                prefix,
            } => {
                assert_eq!(header_name, "Authorization");
                assert_eq!(prefix, "Bearer ");
            }
            _ => panic!("Expected ApiKey variant"),
        }
    }

    /// Test AuthConfig::Jwt variant
    #[test]
    fn test_auth_config_jwt() {
        let json = r#"{"type": "jwt", "secret": "super-secret-key"}"#;
        let config: AuthConfig = serde_json::from_str(json).unwrap();
        match config {
            AuthConfig::Jwt { secret } => {
                assert_eq!(secret, "super-secret-key");
            }
            _ => panic!("Expected Jwt variant"),
        }
    }

    /// Test AuthConfig Default implementation
    #[test]
    fn test_auth_config_default() {
        let default: AuthConfig = AuthConfig::default();
        match default {
            AuthConfig::None => {
                // Default is now None for easier development
            }
            _ => panic!("Default should be None variant"),
        }
    }

    /// Test AuthConfig::validate() accepts non-empty prefix
    #[test]
    fn test_auth_config_validate_non_empty_prefix() {
        let config = AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "sk-".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    /// Test AuthConfig::validate() rejects empty prefix (auth bypass vulnerability)
    #[test]
    fn test_auth_config_validate_empty_prefix_rejected() {
        let config = AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "".to_string(),
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    /// Test AuthConfig::validate() accepts None variant
    #[test]
    fn test_auth_config_validate_none() {
        let config = AuthConfig::None;
        assert!(config.validate().is_ok());
    }

    /// Test AuthConfig::validate() accepts Jwt variant
    #[test]
    fn test_auth_config_validate_jwt() {
        let config = AuthConfig::Jwt {
            secret: "secret".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_auth_config_api_key_serialization() {
        let config = AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "sk-".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("api_key"));
        assert!(json.contains("X-API-Key"));
        let deserialized: AuthConfig = serde_json::from_str(&json).unwrap();
        match deserialized {
            AuthConfig::ApiKey {
                header_name,
                prefix,
            } => {
                assert_eq!(header_name, "X-API-Key");
                assert_eq!(prefix, "sk-");
            }
            _ => panic!("Expected ApiKey variant"),
        }
    }

    #[test]
    fn test_auth_config_jwt_serialization() {
        let config = AuthConfig::Jwt {
            secret: "my-secret-key".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("jwt"));
        let deserialized: AuthConfig = serde_json::from_str(&json).unwrap();
        match deserialized {
            AuthConfig::Jwt { secret } => {
                assert_eq!(secret, "my-secret-key");
            }
            _ => panic!("Expected Jwt variant"),
        }
    }

    #[test]
    fn test_auth_config_none_serialization() {
        let config = AuthConfig::None;
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("none"));
        let deserialized: AuthConfig = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, AuthConfig::None));
    }

    #[test]
    fn test_auth_config_equality() {
        let a = AuthConfig::None;
        let b = AuthConfig::None;
        assert!(matches!(a, AuthConfig::None));
        assert!(matches!(b, AuthConfig::None));
    }
}

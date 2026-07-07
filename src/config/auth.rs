// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Authentication configuration module
//!
//! This module provides authentication-related configuration types.

use serde::{Deserialize, Serialize};

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    ///
    /// This inherent method delegates to the `ValidateConfig` trait impl when
    /// the `validation` feature is enabled, and otherwise runs the same checks
    /// inline. Previously the two implementations diverged: the inherent method
    /// emitted an `eprintln!` warning for short JWT secrets while the trait
    /// impl silently accepted them, so callers observed different behavior
    /// depending on which `validate()` was invoked.
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
            AuthConfig::Jwt { secret } => {
                // Validate JWT secret strength
                if secret.is_empty() {
                    return Err(crate::config::ConfigError::ValidationError(
                        "JWT secret cannot be empty".into(),
                    ));
                }

                // LOW-002: 强制最小密钥长度（256-bit entropy recommended for HS256）
                // 之前 MIN_SECRET_LENGTH=32 定义在 defaults.rs 但未被引用，形同虚设
                let min_len = crate::config::defaults::jwt::MIN_SECRET_LENGTH;
                if secret.len() < min_len {
                    return Err(crate::config::ConfigError::ValidationError(format!(
                        "JWT secret too short: {} chars, minimum {} required (256-bit entropy \
                         recommended). Use `sdforge::security::bearer::generate_secure_jwt_secret()`\
                         to generate a strong secret.",
                        secret.len(),
                        min_len
                    )));
                }

                // Check for obviously weak secrets
                let lower = secret.to_lowercase();
                if lower == "secret"
                    || lower == "password"
                    || lower == "key"
                    || lower == "jwt_secret"
                {
                    return Err(crate::config::ConfigError::ValidationError(
                        "JWT secret is too weak. Avoid using common words like 'secret', \
                         'password', 'key', or 'jwt_secret'. Use a randomly generated value."
                            .into(),
                    ));
                }
            }
            AuthConfig::None => {}
        }
        Ok(())
    }
}

impl crate::config::ValidateConfig for AuthConfig {
    fn validate(&self) -> Result<(), crate::config::ConfigError> {
        // Delegate to the inherent method to guarantee both code paths run
        // identical validation logic. Previously this body was a verbatim
        // copy that diverged (it omitted the eprintln! for short secrets,
        // producing two different behaviors from the same name).
        AuthConfig::validate(self)
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
            secret: "this_is_a_strong_secret_key_1234567890".to_string(), // 32+ chars
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

    // ============================================================================
    // Enhanced JWT Security Validation Tests
    // ============================================================================

    /// Test JWT validation rejects empty secret
    #[test]
    fn test_jwt_validate_empty_secret() {
        let config = AuthConfig::Jwt {
            secret: "".to_string(),
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    /// Test JWT validation rejects short secrets (LOW-002: 强制最小 32 字符)
    #[test]
    fn test_jwt_validate_short_secret_rejected() {
        let config = AuthConfig::Jwt {
            secret: "short".to_string(),
        };
        // LOW-002: 短 secret 现在被拒绝（之前只是 warn 后接受）
        let result = config.validate();
        assert!(
            result.is_err(),
            "short JWT secret should be rejected, got: {:?}",
            result
        );
    }

    /// Test JWT validation rejects weak common secrets
    #[test]
    fn test_jwt_validate_rejects_weak_secrets() {
        // 弱词列表（均 < 32 字符，会先被长度检查拒绝，再被弱词检查拒绝）
        let weak_secrets = vec![
            "secret",
            "SECRET",
            "Secret",
            "password",
            "PASSWORD",
            "key",
            "KEY",
            "jwt_secret",
            "JWT_SECRET",
        ];

        for weak_secret in weak_secrets {
            let config = AuthConfig::Jwt {
                secret: weak_secret.to_string(),
            };
            let result = config.validate();
            assert!(
                result.is_err(),
                "Expected weak secret '{}' to be rejected",
                weak_secret
            );
            // 弱词检查或长度检查都应拒绝（短 secret 命中长度检查，长 secret 命中弱词检查）
            let err = result.unwrap_err();
            assert!(
                err.to_string().contains("weak") || err.to_string().contains("short"),
                "Error should mention 'weak' or 'short': {}",
                err
            );
        }
    }

    /// Test JWT validation accepts strong secrets
    #[test]
    fn test_jwt_validate_accepts_strong_secrets() {
        let strong_secrets = vec![
            "this_is_a_very_long_and_random_secret_key_12345",
            "xK9#mP2$vL5@nQ8*wR3&jT6^yU1!iO4zB7cD0", // 34 chars（≥32）
            "0123456789abcdef0123456789abcdef",      // 32 hex chars = 128 bits
        ];

        for strong_secret in strong_secrets {
            let config = AuthConfig::Jwt {
                secret: strong_secret.to_string(),
            };
            let result = config.validate();
            assert!(
                result.is_ok(),
                "Expected strong secret '{}' ({} chars) to be accepted, got: {:?}",
                strong_secret,
                strong_secret.len(),
                result
            );
        }
    }

    /// Cover the `ValidateConfig for AuthConfig` trait impl (lines 98, 103)
    /// which delegates to the inherent `validate()` method. Existing tests
    /// only call the inherent method, leaving the trait impl body uncovered.
    #[test]
    fn test_validate_config_trait_for_auth_config() {
        use crate::config::ValidateConfig;
        let config = AuthConfig::None;
        assert!(ValidateConfig::validate(&config).is_ok());
    }
}

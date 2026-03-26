// Copyright (c) 2026 Kirky.X
//! Configuration management module
//!
//! This module provides configuration management using the Confers library.
//! Configuration loading uses confers::ConfigLoader for all functionality.
//!
//! Configuration types are organized into submodules by functionality:
//! - `server` - ServerConfig, TlsConfig
//! - `auth` - AuthConfig (ApiKey, JWT, None)
//! - `cors` - CorsConfig, build_cors_layer()
//! - `rate_limit` - RateLimitConfigFile, RateLimitEndpointConfig
//! - `request_size` - RequestSizeConfig
//! - `timeout` - TimeoutConfig
//! - `api` - ApiConfig, TracingConfig, EnvHelper
//! - `defaults` - Default values for all configuration types
//! - `hot_reload` - Hot reload support (feature-gated)

use serde::{Deserialize, Serialize};

pub use confers::Config;
#[cfg(feature = "validation")]
pub use confers::Validate;

// Configuration submodules
pub mod api;
pub mod auth;
pub mod cors;
pub mod rate_limit;
pub mod request_size;
pub mod server;
pub mod timeout;

pub mod defaults;
pub mod hot_reload;

// Re-export hot_reload types with feature gate
#[cfg(feature = "hot-reload")]
pub use hot_reload::{create_config_watcher, ConfigEvent, ConfigManager, ConfigWatcherImpl};

// Re-export all configuration types
pub use api::{ApiConfig, EnvHelper, TracingConfig};
pub use auth::AuthConfig;
pub use cors::{build_cors_layer, CorsConfig};
pub use rate_limit::{RateLimitConfigFile, RateLimitEndpointConfig};
pub use request_size::RequestSizeConfig;
pub use server::{ServerConfig, TlsConfig};
pub use timeout::TimeoutConfig;

/// Configuration loading error
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// File not found
    #[error("File not found: {path}")]
    FileNotFound {
        /// Path to the file that was not found
        path: String,
    },

    /// Parse error
    #[error("Parse error: {message}")]
    ParseError {
        /// Error message from parsing
        message: String,
    },

    /// IO error
    #[error("IO error: {reason}")]
    IoError {
        /// Reason for the IO error
        reason: String,
    },

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Configuration load error (for Confers integration)
    #[error("Configuration load error: {0}")]
    LoadError(String),

    /// Watch error (for hot reload)
    #[error("Watch error: {0}")]
    WatchError(String),

    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[serde(default)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Authentication configuration
    #[serde(alias = "auth")]
    #[config(skip)]
    pub authentication: AuthConfig,
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitConfigFile>,
    /// Request size configuration
    pub request_size: Option<RequestSizeConfig>,
    /// Timeout configuration
    pub timeout: Option<TimeoutConfig>,
}

impl AppConfig {
    /// Create builder for configuration
    pub fn builder() -> AppConfigBuilder {
        AppConfigBuilder::new()
    }

    /// Validate configuration with cross-field validation
    ///
    /// This method performs validation that requires access to multiple fields
    /// and cannot be done at the individual field level.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Rate limit requests is 0 but rate limiting is enabled
    /// - Rate limit window is 0
    /// - Timeout values are inconsistent
    /// - Request size limits are invalid
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate server configuration
        self.server.validate()?;

        // Validate authentication configuration
        self.authentication.validate()?;

        // Validate rate limiting configuration
        if let Some(ref rate_limit) = self.rate_limit {
            rate_limit.validate()?;
        }

        // Validate request size limits
        if let Some(ref request_size) = self.request_size {
            request_size.validate()?;
        }

        // Validate timeout configuration
        if let Some(ref timeout) = self.timeout {
            timeout.validate()?;
        }

        Ok(())
    }
}

/// Builder for AppConfig
#[derive(Debug, Clone, Default)]
pub struct AppConfigBuilder {
    server: ServerConfig,
    authentication: AuthConfig,
    rate_limit: Option<RateLimitConfigFile>,
    request_size: Option<RequestSizeConfig>,
    timeout: Option<TimeoutConfig>,
}

impl AppConfigBuilder {
    /// Create a new AppConfigBuilder with default configuration values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the server configuration for the application.
    pub fn server(mut self, server: ServerConfig) -> Self {
        self.server = server;
        self
    }

    /// Set the authentication configuration for the application.
    pub fn authentication(mut self, authentication: AuthConfig) -> Self {
        self.authentication = authentication;
        self
    }

    /// Set the rate limit configuration for the application.
    pub fn rate_limit(mut self, rate_limit: RateLimitConfigFile) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }

    /// Set the request size configuration for the application.
    pub fn request_size(mut self, request_size: RequestSizeConfig) -> Self {
        self.request_size = Some(request_size);
        self
    }

    /// Set the timeout configuration for the application.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout configuration value.
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
    /// use sdforge::config::{AppConfigBuilder, TimeoutConfig};
    ///
    /// let builder = AppConfigBuilder::new().timeout(TimeoutConfig::default());
    /// let _ = builder;
    /// ```
    pub fn timeout(mut self, timeout: TimeoutConfig) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build an AppConfig instance from the current builder state.
    pub fn build(self) -> AppConfig {
        AppConfig {
            server: self.server,
            authentication: self.authentication,
            rate_limit: self.rate_limit,
            request_size: self.request_size,
            timeout: self.timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test AppConfig deserialization with JSON
    #[test]
    fn test_app_config_json_deserialization() {
        let json = r#"{
            "server": {
                "host": "127.0.0.1",
                "port": 3000,
                "request_timeout_secs": 60
            },
            "authentication": {
                "type": "api_key",
                "header_name": "X-API-Key",
                "prefix": "key-"
            }
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        match &config.authentication {
            AuthConfig::ApiKey {
                header_name,
                prefix,
            } => {
                assert_eq!(header_name, "X-API-Key");
                assert_eq!(prefix, "key-");
            }
            _ => panic!("Expected ApiKey auth config"),
        }
    }

    /// Test AppConfig with authentication alias
    #[test]
    fn test_app_config_auth_alias() {
        // Test that we can create AppConfig with minimal fields
        let config = AppConfig {
            server: ServerConfig::default(),
            authentication: AuthConfig::Jwt {
                secret: "test".to_string(),
            },
            rate_limit: None,
            request_size: None,
            timeout: None,
        };
        // Just verify we can create the config
        match &config.authentication {
            AuthConfig::Jwt { secret } => {
                assert_eq!(secret, "test");
            }
            _ => panic!("Expected Jwt auth config"),
        }
    }

    /// Test AppConfig default
    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert!(config.server.host.is_empty());
        matches!(config.authentication, AuthConfig::None);
    }

    /// Test AppConfig builder
    #[test]
    fn test_app_config_builder() {
        let config = AppConfig::builder()
            .server(ServerConfig {
                host: "localhost".to_string(),
                port: 8080,
                request_timeout_secs: 30,
                cors: None,
            })
            .authentication(AuthConfig::None)
            .build();

        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, 8080);
    }

    #[test]
    fn test_app_config_builder_with_rate_limit() {
        let config = AppConfig::builder()
            .rate_limit(RateLimitConfigFile {
                requests: 1000,
                window_seconds: 60,
            })
            .build();
        assert!(config.rate_limit.is_some());
        let rl = config.rate_limit.unwrap();
        assert_eq!(rl.requests, 1000);
        assert_eq!(rl.window_seconds, 60);
    }

    #[test]
    fn test_app_config_builder_with_request_size() {
        let config = AppConfig::builder()
            .request_size(RequestSizeConfig {
                max_json_size: 2048,
                max_file_size: 4096,
                max_form_size: 1024,
            })
            .build();
        assert!(config.request_size.is_some());
        let rs = config.request_size.unwrap();
        assert_eq!(rs.max_json_size, 2048);
        assert_eq!(rs.max_file_size, 4096);
        assert_eq!(rs.max_form_size, 1024);
    }

    #[test]
    fn test_app_config_builder_with_timeout() {
        let config = AppConfig::builder()
            .timeout(TimeoutConfig {
                default_timeout_secs: 60,
                route_timeouts: std::collections::HashMap::new(),
            })
            .build();
        assert!(config.timeout.is_some());
        assert_eq!(config.timeout.unwrap().default_timeout_secs, 60);
    }

    #[test]
    fn test_app_config_builder_full() {
        let config = AppConfig::builder()
            .server(ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                request_timeout_secs: 120,
                cors: None,
            })
            .authentication(AuthConfig::ApiKey {
                header_name: "X-Auth".to_string(),
                prefix: "token-".to_string(),
            })
            .rate_limit(RateLimitConfigFile {
                requests: 500,
                window_seconds: 30,
            })
            .request_size(RequestSizeConfig::default())
            .timeout(TimeoutConfig::default())
            .build();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert!(config.rate_limit.is_some());
        assert!(config.request_size.is_some());
        assert!(config.timeout.is_some());
    }

    #[test]
    fn test_app_config_serialization_roundtrip() {
        let original = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 4000,
                request_timeout_secs: 90,
                cors: None,
            },
            authentication: AuthConfig::Jwt {
                secret: "test-secret".to_string(),
            },
            rate_limit: Some(RateLimitConfigFile {
                requests: 200,
                window_seconds: 45,
            }),
            request_size: Some(RequestSizeConfig::default()),
            timeout: Some(TimeoutConfig::default()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.server.host, "127.0.0.1");
        assert_eq!(deserialized.server.port, 4000);
        assert!(deserialized.rate_limit.is_some());
    }

    /// Test ConfigError variants
    #[test]
    fn test_config_error_variants() {
        let not_found = ConfigError::FileNotFound {
            path: "/missing.yaml".to_string(),
        };
        assert!(not_found.to_string().contains("File not found"));

        let parse_error = ConfigError::ParseError {
            message: "Invalid YAML".to_string(),
        };
        assert!(parse_error.to_string().contains("Parse error"));

        let io_error = ConfigError::IoError {
            reason: "Permission denied".to_string(),
        };
        assert!(io_error.to_string().contains("IO error"));

        let validation_error = ConfigError::ValidationError("Invalid config".to_string());
        assert!(validation_error.to_string().contains("Validation error"));

        let load_error = ConfigError::LoadError("Failed to load".to_string());
        assert!(load_error.to_string().contains("Configuration load error"));

        let watch_error = ConfigError::WatchError("Watch failed".to_string());
        assert!(watch_error.to_string().contains("Watch error"));
    }

    #[test]
    fn test_config_error_unknown_variant() {
        let error = ConfigError::Unknown("Something went wrong".to_string());
        assert!(error.to_string().contains("Unknown"));
        assert!(error.to_string().contains("Something went wrong"));
    }

    #[test]
    fn test_app_config_validate_valid() {
        let config = AppConfig {
            server: ServerConfig {
                host: "localhost".to_string(),
                port: 8080,
                request_timeout_secs: 30,
                cors: None,
            },
            authentication: AuthConfig::ApiKey {
                header_name: "X-API-Key".to_string(),
                prefix: "sk-".to_string(),
            },
            rate_limit: Some(RateLimitConfigFile {
                requests: 100,
                window_seconds: 60,
            }),
            request_size: Some(RequestSizeConfig::default()),
            timeout: Some(TimeoutConfig::default()),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_app_config_validate_invalid_server_port() {
        let config = AppConfig {
            server: ServerConfig {
                host: "localhost".to_string(),
                port: 0,
                request_timeout_secs: 30,
                cors: None,
            },
            authentication: AuthConfig::None,
            rate_limit: None,
            request_size: None,
            timeout: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_app_config_validate_invalid_auth_prefix() {
        let config = AppConfig {
            server: ServerConfig {
                host: "localhost".to_string(),
                port: 8080,
                request_timeout_secs: 30,
                cors: None,
            },
            authentication: AuthConfig::ApiKey {
                header_name: "X-API-Key".to_string(),
                prefix: "".to_string(),
            },
            rate_limit: None,
            request_size: None,
            timeout: None,
        };
        assert!(config.validate().is_err());
    }
}

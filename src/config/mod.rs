// Copyright (c) 2026 Kirky.X
//! Configuration management module
//!
//! This module provides configuration management using the Confers library.
//! Configuration loading uses confers::ConfigLoader for all functionality.

use serde::{Deserialize, Serialize};

pub use confers::Config;
#[cfg(feature = "validation")]
pub use confers::Validate;

pub mod hot_reload;

// Re-export hot_reload types with feature gate
#[cfg(feature = "hot-reload")]
pub use hot_reload::{create_config_watcher, ConfigEvent, ConfigManager, ConfigWatcherImpl};

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

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[serde(default)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// CORS configuration
    pub cors: Option<CorsConfig>,
}

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
    pub fn validate(&self) -> Result<(), ConfigError> {
        match self {
            AuthConfig::ApiKey { prefix, .. } => {
                if prefix.is_empty() {
                    return Err(ConfigError::ValidationError(
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

/// Logging 配置已移除，日志功能由 inklog 统一管理
/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ApiConfig {
    /// API prefix
    pub prefix: String,
    /// Default version
    pub default_version: String,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct CorsConfig {
    /// Allowed origins
    #[config(skip)]
    pub allowed_origins: Vec<String>,
    /// Allowed methods
    #[config(skip)]
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    #[config(skip)]
    pub allowed_headers: Vec<String>,
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct RateLimitConfigFile {
    /// Requests per window
    pub requests: u32,
    /// Window duration in seconds
    pub window_seconds: u64,
}

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
    1024 * 1024 // 1MB
}

fn default_max_file_size() -> usize {
    100 * 1024 * 1024 // 100MB
}

fn default_max_form_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

/// Timeout configuration for different routes
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[serde(default)]
pub struct TimeoutConfig {
    /// Default request timeout in seconds
    #[serde(default = "default_timeout")]
    #[config(default = default_timeout())]
    pub default_timeout_secs: u64,
    /// Route-specific timeouts
    #[serde(default)]
    #[config(default = default_route_timeouts())]
    #[config(skip)]
    pub route_timeouts: std::collections::HashMap<String, u64>,
}

fn default_timeout() -> u64 {
    30
}

fn default_route_timeouts() -> std::collections::HashMap<String, u64> {
    let mut route_timeouts = std::collections::HashMap::new();
    route_timeouts.insert("/api/upload".to_string(), 300);
    route_timeouts.insert("/api/export".to_string(), 120);
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
}

/// Rate limit endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct RateLimitEndpointConfig {
    /// Endpoint path
    pub path: String,
    /// Rate limit for this endpoint
    pub config: RateLimitConfigFile,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct TlsConfig {
    /// Path to certificate file
    cert_path: String,
    /// Path to private key file
    key_path: String,
}

impl TlsConfig {
    /// Get certificate path
    pub fn cert_path(&self) -> &str {
        &self.cert_path
    }

    /// Get private key path
    pub fn key_path(&self) -> &str {
        &self.key_path
    }
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

/// Build CORS layer from configuration
pub fn build_cors_layer(config: &CorsConfig) -> Result<tower_http::cors::CorsLayer, ConfigError> {
    use tower_http::cors::{Any, CorsLayer};

    // Security: Validate that allowed_origins is not empty
    if config.allowed_origins.is_empty() {
        return Err(ConfigError::ValidationError(
            "CORS allowed_origins cannot be empty. Use explicit origin list or disable CORS".into(),
        ));
    }

    // Validate origin format
    for origin in &config.allowed_origins {
        if !origin.starts_with("http://") && !origin.starts_with("https://") {
            return Err(ConfigError::ValidationError(format!(
                "Invalid CORS origin: {}. Must start with http:// or https://",
                origin
            )));
        }
    }

    let cors = CorsLayer::new().allow_methods(Any).allow_headers(Any);

    // Parse and validate origins
    let origins: Vec<_> = config
        .allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    if origins.is_empty() {
        return Err(ConfigError::ValidationError(
            "No valid origins found in CORS configuration".into(),
        ));
    }

    // Security: Never use Any as origin, always use explicit list
    let cors = cors.allow_origin(origins);

    Ok(cors)
}

impl CorsConfig {
    /// Validate CORS configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check if allowed_origins is empty
        if self.allowed_origins.is_empty() {
            return Err(ConfigError::ValidationError(
                "CORS allowed_origins cannot be empty".into(),
            ));
        }

        // Validate origin format
        for origin in &self.allowed_origins {
            if !origin.starts_with("http://") && !origin.starts_with("https://") {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid CORS origin: {}. Must start with http:// or https://",
                    origin
                )));
            }
        }

        Ok(())
    }
}

/// 日志初始化已移除，日志功能由 sdforge::inklog 统一管理
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

    /// Test ServerConfig Default
    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert!(config.host.is_empty()); // Default String is empty
        assert_eq!(config.port, 0); // Default u16 is 0
        assert_eq!(config.request_timeout_secs, 0); // Default u64 is 0
        assert!(config.cors.is_none());
    }

    /// Test CorsConfig with origins
    #[test]
    fn test_cors_config_with_origins() {
        let json = r#"{
            "allowed_origins": ["http://localhost:3000", "https://example.com"],
            "allowed_methods": ["GET", "POST"],
            "allowed_headers": ["Content-Type", "Authorization"]
        }"#;
        let config: CorsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.allowed_origins.len(), 2);
        assert!(config.allowed_methods.contains(&"GET".to_string()));
        assert!(config.allowed_headers.contains(&"Content-Type".to_string()));
    }

    /// Test build_cors_layer with empty origins
    #[test]
    fn test_build_cors_layer_empty_origins() {
        let config = CorsConfig::default();
        let layer = build_cors_layer(&config);
        // Empty origins should now return an error
        assert!(layer.is_err());
    }
    /// Test build_cors_layer with valid origins
    #[test]
    fn test_build_cors_layer_valid_origins() {
        let json = r#"{"allowed_origins": ["http://localhost:3000"], "allowed_methods": [], "allowed_headers": []}"#;
        let config: CorsConfig = serde_json::from_str(json).unwrap();
        let layer = build_cors_layer(&config);
        assert!(layer.is_ok());
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

    /// Test RateLimitConfigFile
    #[test]
    fn test_rate_limit_config() {
        let json = r#"{"requests": 100, "window_seconds": 60}"#;
        let config: RateLimitConfigFile = serde_json::from_str(json).unwrap();
        assert_eq!(config.requests, 100);
        assert_eq!(config.window_seconds, 60);
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

    /// Test RequestSizeConfig defaults
    #[test]
    fn test_request_size_config_defaults() {
        let config = RequestSizeConfig::default();
        assert_eq!(config.max_json_size, 1024 * 1024);
        assert_eq!(config.max_file_size, 100 * 1024 * 1024);
        assert_eq!(config.max_form_size, 10 * 1024 * 1024);
    }

    /// Test TimeoutConfig defaults and get_timeout
    #[test]
    fn test_timeout_config() {
        let config = TimeoutConfig::default();
        assert_eq!(config.default_timeout_secs, 30);
        assert_eq!(config.get_timeout("/api/upload"), 300);
        assert_eq!(config.get_timeout("/api/export"), 120);
        assert_eq!(config.get_timeout("/unknown"), 30); // Uses default
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
}

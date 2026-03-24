// Copyright (c) 2026 Kirky.X
//! Configuration management module
//!
//! This module provides configuration management using the Confers library.
//! Configuration loading uses confers::ConfigLoader for all functionality.

use serde::{Deserialize, Serialize};

pub use confers::Config;
#[cfg(feature = "validation")]
pub use confers::Validate;

pub mod defaults;
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
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate rate limiting configuration
        if let Some(ref rate_limit) = self.rate_limit {
            if rate_limit.requests == 0 {
                return Err(ConfigError::ValidationError(
                    "rate_limit.requests must be greater than 0 when rate limiting is enabled".into(),
                ));
            }
            if rate_limit.window_seconds == 0 {
                return Err(ConfigError::ValidationError(
                    "rate_limit.window_seconds must be greater than 0".into(),
                ));
            }
            if rate_limit.window_seconds > defaults::rate_limit::MAX_WINDOW_SECS {
                return Err(ConfigError::ValidationError(format!(
                    "rate_limit.window_seconds exceeds maximum allowed value of {} seconds",
                    defaults::rate_limit::MAX_WINDOW_SECS
                )));
            }
        }

        // Validate timeout configuration
        if let Some(ref timeout) = self.timeout {
            if timeout.default_timeout_secs == 0 {
                return Err(ConfigError::ValidationError(
                    "timeout.default_timeout_secs must be greater than 0".into(),
                ));
            }
            // Check for unreasonably long timeouts
            if timeout.default_timeout_secs > 3600 {
                return Err(ConfigError::ValidationError(
                    "timeout.default_timeout_secs should not exceed 3600 seconds (1 hour)".into(),
                ));
            }
        }

        // Validate authentication configuration
        self.authentication.validate()?;

        // Validate request size limits
        if let Some(ref request_size) = self.request_size {
            if request_size.max_json_size == 0 {
                return Err(ConfigError::ValidationError(
                    "request_size.max_json_size must be greater than 0".into(),
                ));
            }
            if request_size.max_file_size == 0 {
                return Err(ConfigError::ValidationError(
                    "request_size.max_file_size must be greater than 0".into(),
                ));
            }
            if request_size.max_form_size == 0 {
                return Err(ConfigError::ValidationError(
                    "request_size.max_form_size must be greater than 0".into(),
                ));
            }
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
    defaults::request_size::MAX_JSON_SIZE
}

fn default_max_file_size() -> usize {
    defaults::request_size::MAX_FILE_SIZE
}

fn default_max_form_size() -> usize {
    defaults::request_size::MAX_FORM_SIZE
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
    defaults::timeout::DEFAULT_TIMEOUT_SECS
}

fn default_route_timeouts() -> std::collections::HashMap<String, u64> {
    let mut route_timeouts = std::collections::HashMap::new();
    route_timeouts.insert("/api/upload".to_string(), defaults::timeout::UPLOAD_TIMEOUT_SECS);
    route_timeouts.insert("/api/export".to_string(), defaults::timeout::EXPORT_TIMEOUT_SECS);
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
    fn test_server_config_serialization() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 9000,
            request_timeout_secs: 45,
            cors: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.host, "localhost");
        assert_eq!(deserialized.port, 9000);
        assert_eq!(deserialized.request_timeout_secs, 45);
    }

    #[test]
    fn test_server_config_with_cors() {
        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: Some(CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
                allowed_methods: vec!["GET".to_string()],
                allowed_headers: vec!["Authorization".to_string()],
            }),
        };
        assert!(config.cors.is_some());
        let cors = config.cors.unwrap();
        assert_eq!(cors.allowed_origins.len(), 1);
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
    }

    #[test]
    fn test_rate_limit_config_zero_window() {
        let config = RateLimitConfigFile {
            requests: 100,
            window_seconds: 0,
        };
        assert_eq!(config.window_seconds, 0);
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
    fn test_cors_config_validate_empty_origins() {
        let config = CorsConfig {
            allowed_origins: vec![],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec![],
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_cors_config_validate_invalid_origin_no_scheme() {
        let config = CorsConfig {
            allowed_origins: vec!["localhost:3000".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec![],
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid CORS origin"));
    }

    #[test]
    fn test_cors_config_validate_invalid_origin_http_only() {
        let config = CorsConfig {
            allowed_origins: vec!["http://".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec![],
        };
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_cors_config_validate_valid_origins() {
        let config = CorsConfig {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "https://example.com".to_string(),
            ],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_tls_config_getters() {
        let config = TlsConfig {
            cert_path: "/etc/ssl/cert.pem".to_string(),
            key_path: "/etc/ssl/key.pem".to_string(),
        };
        assert_eq!(config.cert_path(), "/etc/ssl/cert.pem");
        assert_eq!(config.key_path(), "/etc/ssl/key.pem");
    }

    #[test]
    fn test_tls_config_serialization() {
        let config = TlsConfig {
            cert_path: "/path/to/cert.pem".to_string(),
            key_path: "/path/to/key.pem".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: TlsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cert_path(), "/path/to/cert.pem");
        assert_eq!(deserialized.key_path(), "/path/to/key.pem");
    }

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

    #[test]
    fn test_config_error_unknown_variant() {
        let error = ConfigError::Unknown("Something went wrong".to_string());
        assert!(error.to_string().contains("Unknown"));
        assert!(error.to_string().contains("Something went wrong"));
    }

    #[test]
    fn test_default_timeout_function() {
        assert_eq!(default_timeout(), 30);
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

    #[test]
    fn test_default_route_timeouts_function() {
        let route_timeouts = default_route_timeouts();
        assert_eq!(route_timeouts.get("/api/upload"), Some(&300));
        assert_eq!(route_timeouts.get("/api/export"), Some(&120));
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

    #[test]
    fn test_auth_config_equality() {
        let a = AuthConfig::None;
        let b = AuthConfig::None;
        assert!(matches!(a, AuthConfig::None));
        assert!(matches!(b, AuthConfig::None));
    }

    #[test]
    fn test_cors_config_clone() {
        let config = CorsConfig {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["Authorization".to_string()],
        };
        let cloned = config.clone();
        assert_eq!(cloned.allowed_origins, config.allowed_origins);
        assert_eq!(cloned.allowed_methods, config.allowed_methods);
    }
}

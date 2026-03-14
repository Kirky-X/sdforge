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
    /// Database configuration
    pub database: DatabaseConfig,
    /// Authentication configuration
    #[serde(alias = "auth")]
    #[config(skip)]
    pub authentication: AuthConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
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
    database: DatabaseConfig,
    authentication: AuthConfig,
    logging: LoggingConfig,
    rate_limit: Option<RateLimitConfigFile>,
    request_size: Option<RequestSizeConfig>,
    timeout: Option<TimeoutConfig>,
}

impl AppConfigBuilder {
    /// Create a new AppConfigBuilder with default configuration values.
    ///
    /// # Returns
    ///
    /// Returns a builder initialized with default configuration components.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::config::AppConfigBuilder;
    ///
    /// let builder = AppConfigBuilder::new();
    /// let _ = builder;
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the server configuration for the application.
    ///
    /// # Arguments
    ///
    /// * `server` - Server configuration value.
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
    /// use sdforge::config::{AppConfigBuilder, ServerConfig};
    ///
    /// let builder = AppConfigBuilder::new().server(ServerConfig::default());
    /// let _ = builder;
    /// ```
    pub fn server(mut self, server: ServerConfig) -> Self {
        self.server = server;
        self
    }

    /// Set the database configuration for the application.
    ///
    /// # Arguments
    ///
    /// * `database` - Database configuration value.
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
    /// use sdforge::config::{AppConfigBuilder, DatabaseConfig};
    ///
    /// let builder = AppConfigBuilder::new().database(DatabaseConfig::default());
    /// let _ = builder;
    /// ```
    pub fn database(mut self, database: DatabaseConfig) -> Self {
        self.database = database;
        self
    }

    /// Set the authentication configuration for the application.
    ///
    /// # Arguments
    ///
    /// * `authentication` - Authentication configuration value.
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
    /// use sdforge::config::{AppConfigBuilder, AuthConfig};
    ///
    /// let builder = AppConfigBuilder::new().authentication(AuthConfig::default());
    /// let _ = builder;
    /// ```
    pub fn authentication(mut self, authentication: AuthConfig) -> Self {
        self.authentication = authentication;
        self
    }

    /// Set the logging configuration for the application.
    ///
    /// # Arguments
    ///
    /// * `logging` - Logging configuration value.
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
    /// use sdforge::config::{AppConfigBuilder, LoggingConfig};
    ///
    /// let builder = AppConfigBuilder::new().logging(LoggingConfig::default());
    /// let _ = builder;
    /// ```
    pub fn logging(mut self, logging: LoggingConfig) -> Self {
        self.logging = logging;
        self
    }

    /// Set the rate limit configuration for the application.
    ///
    /// # Arguments
    ///
    /// * `rate_limit` - Rate limit configuration value.
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
    /// use sdforge::config::{AppConfigBuilder, RateLimitConfigFile};
    ///
    /// let builder = AppConfigBuilder::new().rate_limit(RateLimitConfigFile::default());
    /// let _ = builder;
    /// ```
    pub fn rate_limit(mut self, rate_limit: RateLimitConfigFile) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }

    /// Set the request size configuration for the application.
    ///
    /// # Arguments
    ///
    /// * `request_size` - Request size configuration value.
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
    /// use sdforge::config::{AppConfigBuilder, RequestSizeConfig};
    ///
    /// let builder = AppConfigBuilder::new().request_size(RequestSizeConfig::default());
    /// let _ = builder;
    /// ```
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
    ///
    /// # Returns
    ///
    /// Returns a fully constructed AppConfig.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::config::AppConfigBuilder;
    ///
    /// let config = AppConfigBuilder::new().build();
    /// let _ = config;
    /// ```
    pub fn build(self) -> AppConfig {
        AppConfig {
            server: self.server,
            database: self.database,
            authentication: self.authentication,
            logging: self.logging,
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

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Database connection string
    connection_string: String,
    /// Maximum connections
    max_connections: u32,
}

impl DatabaseConfig {
    /// Get database connection string
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// Get maximum connections
    pub fn max_connections(&self) -> u32 {
        self.max_connections
    }
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

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Output format
    pub format: String,
}

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

/// Initialize logging
pub fn init_logging(_config: &LoggingConfig) {
    #[cfg(feature = "logging")]
    {
        use std::str::FromStr;
        use tracing::Level;
        use tracing_subscriber::fmt;

        let level = if _config.level.trim().is_empty() {
            Level::INFO
        } else {
            Level::from_str(_config.level.trim()).unwrap_or(Level::INFO)
        };
        let format = _config.format.trim().to_lowercase();

        let base_builder = fmt().with_max_level(level);
        let init_result = match format.as_str() {
            "json" => base_builder.json().try_init(),
            "compact" => base_builder.compact().try_init(),
            "pretty" => base_builder.pretty().try_init(),
            _ => base_builder.pretty().try_init(),
        };
        let _ = init_result;
    }
    #[cfg(not(feature = "logging"))]
    {
        let _ = _config;
    }
}

/// Initialize logging with default settings
pub fn init_logging_default() {
    #[cfg(feature = "logging")]
    {
        let config = LoggingConfig {
            level: "info".to_string(),
            format: "pretty".to_string(),
        };
        init_logging(&config);
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
            "database": {
                "connection_string": "mysql://localhost/mydb",
                "max_connections": 25
            },
            "authentication": {
                "type": "api_key",
                "header_name": "X-API-Key",
                "prefix": "key-"
            },
            "logging": {
                "level": "debug",
                "format": "pretty"
            }
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.database.max_connections(), 25);
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
            database: DatabaseConfig::default(),
            authentication: AuthConfig::Jwt {
                secret: "test".to_string(),
            },
            logging: LoggingConfig::default(),
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

    /// Test DatabaseConfig Default
    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert!(config.connection_string().is_empty());
        assert_eq!(config.max_connections(), 0); // Default u32 is 0
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
        assert!(config.database.connection_string().is_empty());
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
            .database(DatabaseConfig {
                connection_string: "postgres://localhost/db".to_string(),
                max_connections: 10,
            })
            .authentication(AuthConfig::None)
            .logging(LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            })
            .build();

        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, 8080);
        assert_eq!(
            config.database.connection_string(),
            "postgres://localhost/db"
        );
        assert_eq!(config.database.max_connections(), 10);
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
}

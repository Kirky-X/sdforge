// Copyright (c) 2026 Kirky.X
//! Configuration management module

use serde::Deserialize;

pub mod hot_reload;

// Re-export hot_reload types with feature gate
#[cfg(feature = "hot-reload")]
pub use hot_reload::ConfigWatcher;

/// Configuration event type
#[derive(Debug, Clone)]
pub struct ConfigEvent {
    /// Event type
    pub event_type: String,
    /// Path that changed
    pub path: String,
}

/// Configuration loading error
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Parse error: {message}")]
    ParseError { message: String },

    #[error("IO error: {reason}")]
    IoError { reason: String },

    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// Application configuration placeholder
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Authentication configuration
    #[serde(alias = "auth")]
    pub authentication: AuthConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitConfigFile>,
}

/// Server configuration
#[derive(Debug, Clone, Default, Deserialize)]
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

/// Database configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection string
    pub connection_string: String,
    /// Maximum connections
    pub max_connections: u32,
}

/// Authentication configuration
#[derive(Debug, Clone, Deserialize)]
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
    /// OAuth2 authentication (not yet implemented)
    #[serde(rename = "oauth2")]
    OAuth2,
}

impl Default for AuthConfig {
    fn default() -> Self {
        // Default to JWT with empty secret (will be overridden by config)
        AuthConfig::Jwt {
            secret: String::new(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Output format
    pub format: String,
}

/// API configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ApiConfig {
    /// API prefix
    pub prefix: String,
    /// Default version
    pub default_version: String,
}

/// CORS configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: Vec<String>,
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    pub allowed_headers: Vec<String>,
}

/// Rate limit configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RateLimitConfigFile {
    /// Requests per window
    pub requests: u32,
    /// Window duration in seconds
    pub window_seconds: u64,
}

/// Rate limit endpoint configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RateLimitEndpointConfig {
    /// Endpoint path
    pub path: String,
    /// Rate limit for this endpoint
    pub config: RateLimitConfigFile,
}

/// TLS configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TlsConfig {
    /// Path to certificate file
    pub cert_path: String,
    /// Path to private key file
    pub key_path: String,
}

/// Tracing configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TracingConfig {
    /// Tracing enabled
    pub enabled: bool,
}

/// Environment helper
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EnvHelper {
    /// Environment name
    pub environment: String,
}

/// Configuration loader
#[derive(Debug, Clone, Default)]
pub struct ConfigLoader {
    /// Configuration file path
    pub path: String,
}

impl ConfigLoader {
    /// Create a new configuration loader
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    /// Load configuration from file
    pub fn load(&self) -> Result<AppConfig, ConfigError> {
        Ok(AppConfig::default())
    }
}

/// Build CORS layer from configuration
pub fn build_cors_layer(config: &CorsConfig) -> Result<tower_http::cors::CorsLayer, ConfigError> {
    use tower_http::cors::{Any, CorsLayer};

    let cors = CorsLayer::new().allow_methods(Any).allow_headers(Any);

    let cors = if !config.allowed_origins.is_empty() {
        let origins: Vec<_> = config
            .allowed_origins
            .iter()
            .filter_map(|origin| origin.parse().ok())
            .collect();

        if !origins.is_empty() {
            cors.allow_origin(origins)
        } else {
            cors.allow_origin(Any)
        }
    } else {
        cors.allow_origin(Any)
    };

    Ok(cors)
}

/// Initialize logging
pub fn init_logging(_config: &LoggingConfig) {
    // Placeholder - would set up tracing subscriber
}

/// Initialize logging with default settings
pub fn init_logging_default() {
    // Placeholder - would set up default logging
}

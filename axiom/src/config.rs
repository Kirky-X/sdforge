//! Configuration module for the Axiom framework
//!
//! This module provides configuration loading and management capabilities.
//! Requires the `http` feature.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::HashMap;
use std::path::PathBuf;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// TLS configuration
    #[serde(default)]
    pub tls: Option<TlsConfig>,
    /// CORS configuration
    #[serde(default)]
    pub cors: Option<CorsConfig>,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to TLS certificate
    pub cert_path: PathBuf,
    /// Path to TLS private key
    pub key_path: PathBuf,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    /// Allowed methods
    #[serde(default)]
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    #[serde(default)]
    pub allowed_headers: Vec<String>,
    /// Allow credentials
    #[serde(default)]
    pub allow_credentials: bool,
    /// Max age in seconds
    #[serde(default)]
    pub max_age: Option<u64>,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// API name
    pub name: String,
    /// API version
    pub version: String,
    /// API description
    pub description: Option<String>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DatabaseConfig {
    /// SQLite database
    #[serde(rename = "sqlite")]
    Sqlite {
        /// Database path
        path: PathBuf,
    },
    /// PostgreSQL database
    #[serde(rename = "postgresql")]
    Postgresql {
        /// Connection string
        connection_string: String,
        /// Max connections
        max_connections: u32,
    },
    /// Redis configuration
    #[serde(rename = "redis")]
    Redis {
        /// Redis connection string
        connection_string: String,
        /// Pool size
        pool_size: u32,
    },
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfigFile {
    /// Max requests per window
    pub max_requests: u32,
    /// Window duration in seconds
    pub window_seconds: u32,
    /// Per-endpoint rate limits
    #[serde(default)]
    pub endpoints: HashMap<String, RateLimitEndpointConfig>,
}

/// Per-endpoint rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEndpointConfig {
    /// Max requests for this endpoint
    pub max_requests: u32,
    /// Window duration in seconds for this endpoint
    pub window_seconds: u32,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthConfig {
    /// API key authentication
    #[serde(rename = "api_key")]
    ApiKey {
        /// Header name
        header_name: String,
        /// Prefix (e.g., "Bearer " or "ApiKey ")
        prefix: String,
    },
    /// JWT authentication
    #[serde(rename = "jwt")]
    Jwt {
        /// JWT secret
        secret: String,
        /// Token expiration in seconds
        expiration_seconds: u64,
        /// Issuer
        issuer: Option<String>,
    },
    /// OAuth2 configuration
    #[serde(rename = "oauth2")]
    OAuth2 {
        /// Client ID
        client_id: String,
        /// Client secret
        client_secret: String,
        /// Token URL
        token_url: String,
        /// Authorization URL
        auth_url: String,
    },
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Log format (json or text)
    #[serde(default = "default_format")]
    pub format: String,
    /// Output file
    pub output_file: Option<PathBuf>,
}

fn default_format() -> String {
    "text".to_string()
}

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Service name
    pub service_name: String,
    /// OTLP exporter endpoint
    pub otlp_endpoint: Option<String>,
    /// Sample rate
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

fn default_sample_rate() -> f64 {
    1.0
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// API metadata
    pub api: ApiConfig,
    /// Database configuration
    #[serde(default)]
    pub database: Option<DatabaseConfig>,
    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfigFile>,
    /// Authentication configuration
    #[serde(default)]
    pub authentication: Option<AuthConfig>,
    /// Logging configuration
    #[serde(default)]
    pub logging: Option<LoggingConfig>,
    /// Tracing configuration
    #[serde(default)]
    pub tracing: Option<TracingConfig>,
    /// Custom configuration sections
    #[serde(default)]
    pub custom: HashMap<String, toml::Value>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                tls: None,
                cors: None,
            },
            api: ApiConfig {
                name: "axiom-api".to_string(),
                version: "0.1.0".to_string(),
                description: None,
            },
            database: None,
            rate_limit: None,
            authentication: None,
            logging: None,
            tracing: None,
            custom: HashMap::new(),
        }
    }
}

/// Configuration loader
#[derive(Debug, Clone)]
pub struct ConfigLoader {
    /// Configuration path
    path: PathBuf,
    /// Environment prefix
    env_prefix: String,
}

impl ConfigLoader {
    /// Create new configuration loader
    pub fn new(path: impl Into<PathBuf>, env_prefix: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            env_prefix: env_prefix.into(),
        }
    }

    /// Load configuration from file
    pub fn load(&self) -> Result<AppConfig, ConfigError> {
        // Load from file
        let config_str = std::fs::read_to_string(&self.path)
            .map_err(|e| ConfigError::LoadError(self.path.clone(), e))?;

        // Parse TOML
        let mut config: AppConfig = toml::from_str(&config_str)
            .map_err(ConfigError::ParseError)?;

        // Apply environment overrides
        self.apply_env_overrides(&mut config);

        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&self, config: &mut AppConfig) {
        // Server overrides
        if let Ok(host) = std::env::var(&format!("{}_SERVER_HOST", self.env_prefix)) {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var(&format!("{}_SERVER_PORT", self.env_prefix)) {
            if let Ok(port_num) = port.parse() {
                config.server.port = port_num;
            }
        }

        // API overrides
        if let Ok(version) = std::env::var(&format!("{}_API_VERSION", self.env_prefix)) {
            config.api.version = version;
        }

        // Database overrides
        if let Ok(conn_str) = std::env::var(&format!("{}_DATABASE_URL", self.env_prefix)) {
            if let Some(db) = &mut config.database {
                match db {
                    DatabaseConfig::Postgresql { connection_string, .. } => {
                        *connection_string = conn_str;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Get configuration path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load configuration from {0}: {1}")]
    LoadError(PathBuf, std::io::Error),

    #[error("Failed to parse configuration: {0}")]
    ParseError(toml::de::Error),

    #[error("Configuration validation failed: {0}")]
    ValidationError(String),
}

/// Environment helper
#[derive(Debug, Clone)]
pub struct EnvHelper {
    /// Environment prefix
    prefix: String,
}

impl EnvHelper {
    /// Create new environment helper
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    /// Get a string environment variable
    pub fn get(&self, key: &str) -> Option<String> {
        std::env::var(&format!("{}_{}", self.prefix, key)).ok()
    }

    /// Get a u16 environment variable
    pub fn get_u16(&self, key: &str) -> Option<u16> {
        self.get(key).and_then(|s| s.parse().ok())
    }

    /// Get a bool environment variable
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|s| s.parse().ok())
    }
}

/// Initialize logging based on configuration
///
/// This function sets up logging using env_logger or tracing depending on configuration.
/// # Arguments
/// * `config` - The logging configuration
/// # Example
/// ```ignore
/// use axiom::config::{LoggingConfig, init_logging};
///
/// let config = LoggingConfig {
///     level: "info".to_string(),
///     format: "text".to_string(),
///     output_file: None,
/// };
///
/// init_logging(&config);
/// ```
#[cfg(feature = "logging")]
pub fn init_logging(config: &LoggingConfig) {
    use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};
    use tracing_appender::non_blocking::NonBlocking;

    // Set RUST_LOG from config level
    std::env::set_var("RUST_LOG", &config.level);

    // Create output (file or stdout)
    let non_blocking: NonBlocking = if let Some(ref output_file) = config.output_file {
        let file = std::fs::File::create(output_file)
            .expect("Failed to create log file");
        tracing_appender::non_blocking(file).0
    } else {
        tracing_appender::non_blocking(std::io::stdout()).0
    };

    // Set up the subscriber
    let subscriber = tracing_subscriber::registry();

    // Create the formatting layer
    let layer = fmt::layer().with_writer(non_blocking);
    let subscriber = subscriber.with(layer);

    // Add env filter for log level
    let env_filter = EnvFilter::from_default_env();
    let subscriber = subscriber.with(env_filter);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    // Log initialization message
    tracing::info!(
        target: "axiom::config",
        "Logging initialized at level: {}, format: {}",
        config.level,
        config.format
    );
}

/// Initialize logging with default settings
///
/// Uses INFO level and text format by default.
#[cfg(feature = "logging")]
pub fn init_logging_default() {
    let config = LoggingConfig {
        level: "info".to_string(),
        format: "text".to_string(),
        output_file: None,
    };
    init_logging(&config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.api.name, "axiom-api");
        assert_eq!(config.api.version, "0.1.0");
    }

    #[test]
    fn test_env_helper() {
        std::env::set_var("TEST_APP_FOO", "bar");
        let helper = EnvHelper::new("TEST_APP");
        assert_eq!(helper.get("FOO"), Some("bar".to_string()));
        assert_eq!(helper.get_u16("BAR"), None);
        std::env::remove_var("TEST_APP_FOO");
    }

    #[test]
    fn test_cors_config_defaults() {
        let config = CorsConfig {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
            allow_credentials: false,
            max_age: None,
        };
        assert!(config.allowed_origins.contains(&"*".to_string()));
    }
}

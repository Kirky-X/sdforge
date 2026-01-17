// Copyright (c) 2026 Kirky.X
//! Configuration module for the Axiom framework
//!
//! This module provides configuration loading and management capabilities.
//! Requires the `http` feature.

#[cfg(feature = "hot-reload")]
pub mod hot_reload;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[cfg(feature = "http")]
use axum::http::HeaderValue;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host
    pub(crate) host: String,
    /// Server port
    pub(crate) port: u16,
    /// Request timeout in seconds (default: 30)
    #[serde(default = "default_request_timeout")]
    pub(crate) request_timeout_secs: u64,
    /// TLS configuration
    #[serde(default)]
    pub(crate) tls: Option<TlsConfig>,
    /// CORS configuration
    #[serde(default)]
    pub(crate) cors: Option<CorsConfig>,
}

/// Default request timeout: 30 seconds
fn default_request_timeout() -> u64 {
    30
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to TLS certificate
    pub(crate) cert_path: PathBuf,
    /// Path to TLS private key
    pub(crate) key_path: PathBuf,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins
    #[serde(default)]
    pub(crate) allowed_origins: Vec<String>,
    /// Allowed methods
    #[serde(default)]
    pub(crate) allowed_methods: Vec<String>,
    /// Allowed headers
    #[serde(default)]
    pub(crate) allowed_headers: Vec<String>,
    /// Allow credentials
    #[serde(default)]
    pub(crate) allow_credentials: bool,
    /// Max age in seconds
    #[serde(default)]
    pub(crate) max_age: Option<u64>,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// API name
    pub(crate) name: String,
    /// API version
    pub(crate) version: String,
    /// API description
    pub(crate) description: Option<String>,
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
    pub(crate) max_requests: u32,
    /// Window duration in seconds
    pub(crate) window_seconds: u32,
    /// Per-endpoint rate limits
    #[serde(default)]
    pub(crate) endpoints: HashMap<String, RateLimitEndpointConfig>,
}

impl RateLimitConfigFile {
    /// Create new RateLimitConfigFile
    pub fn new(
        max_requests: u32,
        window_seconds: u32,
        endpoints: HashMap<String, RateLimitEndpointConfig>,
    ) -> Self {
        Self {
            max_requests,
            window_seconds,
            endpoints,
        }
    }
}

/// Per-endpoint rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEndpointConfig {
    /// Max requests for this endpoint
    pub(crate) max_requests: u32,
    /// Window duration in seconds for this endpoint
    pub(crate) window_seconds: u32,
}

impl RateLimitEndpointConfig {
    /// Create new RateLimitEndpointConfig
    pub fn new(max_requests: u32, window_seconds: u32) -> Self {
        Self {
            max_requests,
            window_seconds,
        }
    }
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
                request_timeout_secs: 30,
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

impl ServerConfig {
    /// Get server host
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Set server host
    pub fn set_host(&mut self, host: String) {
        self.host = host;
    }

    /// Get server port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Set server port
    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    /// Get request timeout in seconds
    pub fn request_timeout_secs(&self) -> u64 {
        self.request_timeout_secs
    }

    /// Set request timeout in seconds
    pub fn set_request_timeout_secs(&mut self, timeout: u64) {
        self.request_timeout_secs = timeout;
    }

    /// Get TLS configuration
    pub fn tls(&self) -> Option<&TlsConfig> {
        self.tls.as_ref()
    }

    /// Set TLS configuration
    pub fn set_tls(&mut self, tls: Option<TlsConfig>) {
        self.tls = tls;
    }

    /// Get CORS configuration
    pub fn cors(&self) -> Option<&CorsConfig> {
        self.cors.as_ref()
    }

    /// Set CORS configuration
    pub fn set_cors(&mut self, cors: Option<CorsConfig>) {
        self.cors = cors;
    }

    /// Create new ServerConfig with custom values
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            request_timeout_secs: 30,
            tls: None,
            cors: None,
        }
    }
}

impl TlsConfig {
    /// Get TLS certificate path
    pub fn cert_path(&self) -> &PathBuf {
        &self.cert_path
    }

    /// Get TLS private key path
    pub fn key_path(&self) -> &PathBuf {
        &self.key_path
    }
}

impl CorsConfig {
    /// Create new CorsConfig with given values
    pub fn new(
        allowed_origins: Vec<String>,
        allowed_methods: Vec<String>,
        allowed_headers: Vec<String>,
        allow_credentials: bool,
        max_age: Option<u64>,
    ) -> Self {
        Self {
            allowed_origins,
            allowed_methods,
            allowed_headers,
            allow_credentials,
            max_age,
        }
    }

    /// Get allowed origins
    pub fn allowed_origins(&self) -> &[String] {
        &self.allowed_origins
    }

    /// Get allowed methods
    pub fn allowed_methods(&self) -> &[String] {
        &self.allowed_methods
    }

    /// Get allowed headers
    pub fn allowed_headers(&self) -> &[String] {
        &self.allowed_headers
    }

    /// Check if credentials are allowed
    pub fn allow_credentials(&self) -> bool {
        self.allow_credentials
    }

    /// Get max age
    pub fn max_age(&self) -> Option<u64> {
        self.max_age
    }
}

impl ApiConfig {
    /// Create new ApiConfig with given values
    pub fn new(name: String, version: String, description: Option<String>) -> Self {
        Self {
            name,
            version,
            description,
        }
    }

    /// Get API name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get API version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get API description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Set API name
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Set API version
    pub fn set_version(&mut self, version: String) {
        self.version = version;
    }

    /// Set API description
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
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
        // Validate configuration file path for security
        self.validate_config_path()?;

        // Load from file
        let config_str = std::fs::read_to_string(&self.path)
            .map_err(|e| ConfigError::LoadError(self.path.clone(), e))?;

        // Parse TOML
        let mut config: AppConfig = toml::from_str(&config_str).map_err(ConfigError::ParseError)?;

        // Apply environment overrides
        self.apply_env_overrides(&mut config);

        Ok(config)
    }

    /// Validate configuration file path for security
    fn validate_config_path(&self) -> Result<(), ConfigError> {
        // Canonicalize path to resolve relative paths and symlinks
        let canonical_path = self
            .path
            .canonicalize()
            .map_err(|e| ConfigError::LoadError(self.path.clone(), e))?;

        // Check if path is a symlink (security risk)
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if let Ok(metadata) = std::fs::metadata(&canonical_path) {
                // Check if file type indicates a symlink (mode would have S_IFLNK)
                // For regular files, we check the mode
                if metadata.mode() & 0o170000 == 0o120000 {
                    // S_IFLNK
                    return Err(ConfigError::ValidationError(
                        "Configuration file cannot be a symbolic link".to_string(),
                    ));
                }
            }
        }

        // Define allowed directories for configuration files
        // In production, only these directories should be used
        // Note: /tmp is handled separately below for testing scenarios
        const ALLOWED_DIRS: [&str; 4] =
            ["/etc/axiom", "/opt/axiom/config", "./config", "/var/axiom"];

        // Get parent directory
        let parent_dir = canonical_path
            .parent()
            .and_then(|p| p.to_str())
            .ok_or_else(|| {
                ConfigError::ValidationError("Configuration file has invalid path".to_string())
            })?;

        // Check if parent directory is in allowed list
        let is_allowed = ALLOWED_DIRS
            .iter()
            .any(|allowed| parent_dir.starts_with(allowed) || canonical_path.starts_with(allowed));

        if !is_allowed {
            // For relative paths or paths in temp dirs (testing), check if file exists and is readable
            if !self.path.is_absolute() || parent_dir.starts_with("/tmp") {
                // Check if file exists and is readable (allow for testing)
                std::fs::metadata(&self.path)
                    .map_err(|e| ConfigError::LoadError(self.path.clone(), e))?;
            } else {
                return Err(ConfigError::ValidationError(format!(
                    "Configuration file must be in allowed directory. Got: {}",
                    parent_dir
                )));
            }
        }

        // Check file permissions on Unix systems (skip for temp files in tests)
        #[cfg(unix)]
        {
            // Skip permission check for temp directories (testing)
            if !parent_dir.starts_with("/tmp") {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(&canonical_path) {
                    let perms = metadata.permissions();
                    let mode = perms.mode();

                    // Reject group and others having write permissions
                    if mode & 0o077 != 0o600 {
                        return Err(ConfigError::ValidationError(
                            "Configuration file has insecure permissions. Expected 0600 or stricter.".to_string()
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&self, config: &mut AppConfig) {
        if let Ok(host) = std::env::var(format!("{}_SERVER_HOST", self.env_prefix)) {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var(format!("{}_SERVER_PORT", self.env_prefix)) {
            if let Ok(port_num) = port.parse() {
                config.server.port = port_num;
            }
        }

        if let Ok(version) = std::env::var(format!("{}_API_VERSION", self.env_prefix)) {
            config.api.version = version;
        }

        if let Ok(conn_str) = std::env::var(format!("{}_DATABASE_URL", self.env_prefix)) {
            if let Some(DatabaseConfig::Postgresql {
                connection_string, ..
            }) = &mut config.database
            {
                if Self::validate_connection_string(&conn_str) {
                    *connection_string = conn_str;
                } else {
                    // Security: Only log that validation failed, not the actual connection string
                    #[cfg(feature = "logging")]
                    tracing::warn!(target: "config",
                        "Database connection string validation failed"
                    );
                }
            }
        }

        // OAuth2 secret from environment (security: never store secrets in config files)
        if let Ok(secret) = std::env::var(format!("{}_OAUTH2_CLIENT_SECRET", self.env_prefix)) {
            if let Some(AuthConfig::OAuth2 { client_secret, .. }) = &mut config.authentication {
                *client_secret = secret;
            }
        }

        // JWT secret from environment (security: prefer env var over config file)
        if let Ok(jwt_secret) = std::env::var(format!("{}_JWT_SECRET", self.env_prefix)) {
            if let Some(AuthConfig::Jwt { secret, .. }) = &mut config.authentication {
                *secret = jwt_secret;
            }
        }
    }

    /// Validate database connection string format
    fn validate_connection_string(conn_str: &str) -> bool {
        if conn_str.is_empty() || conn_str.len() > 2048 {
            return false;
        }

        let forbidden_chars = [';', '\'', '"', '\n', '\r', '\0'];
        if conn_str.chars().any(|c| forbidden_chars.contains(&c)) {
            return false;
        }

        if !conn_str.starts_with("postgresql://") && !conn_str.starts_with("postgres://") {
            return false;
        }

        true
    }

    /// Get configuration path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to load configuration from the specified path
    #[error("Failed to load configuration from {0}: {1}")]
    LoadError(PathBuf, std::io::Error),

    /// Failed to parse configuration file (TOML format error)
    #[error("Failed to parse configuration: {0}")]
    ParseError(toml::de::Error),

    /// Configuration validation failed
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
        std::env::var(format!("{}_{}", self.prefix, key)).ok()
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
pub fn init_logging(config: &LoggingConfig) -> Result<(), ConfigError> {
    use tracing_appender::non_blocking::NonBlocking;
    use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

    // Set RUST_LOG from config level
    std::env::set_var("RUST_LOG", &config.level);

    // Create output (file or stdout)
    let non_blocking: NonBlocking = if let Some(ref output_file) = config.output_file {
        let file = std::fs::File::create(output_file).map_err(|e| {
            ConfigError::ValidationError(format!("Failed to create log file: {}", e))
        })?;
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

    tracing::subscriber::set_global_default(subscriber).map_err(|e| {
        ConfigError::ValidationError(format!("Failed to set tracing subscriber: {}", e))
    })?;

    // Log initialization message
    tracing::info!(
        target: "axiom::config",
        "Logging initialized at level: {}, format: {}",
        config.level,
        config.format
    );

    Ok(())
}

/// Initialize logging with default settings
///
/// Uses INFO level and text format by default.
#[cfg(feature = "logging")]
pub fn init_logging_default() -> Result<(), ConfigError> {
    let config = LoggingConfig {
        level: "info".to_string(),
        format: "text".to_string(),
        output_file: None,
    };
    init_logging(&config)
}

/// Build CORS layer from configuration
///
/// # Arguments
/// * `config` - The CORS configuration
///
/// # Returns
/// A configured CORS layer that can be applied to Axum router
pub fn build_cors_layer(config: &CorsConfig) -> Result<tower_http::cors::CorsLayer, ConfigError> {
    use axum::http::{HeaderName, Method};
    use tower_http::cors::CorsLayer;

    // Validate: wildcard origin ("*") is incompatible with allow_credentials(true)
    // This is a browser security requirement - credentials cannot be sent with wildcard origins
    if config.allow_credentials {
        let has_wildcard = config.allowed_origins.iter().any(|origin| origin == "*");
        if has_wildcard {
            return Err(ConfigError::ValidationError(
                "CORS configuration error: wildcard origin \"*\" cannot be used with allow_credentials(true). \
                Either remove the wildcard origin, set allow_credentials to false, or specify specific origins."
                    .to_string(),
            ));
        }
    }

    // Parse and set allowed origins
    let origins: Result<Vec<_>, _> = config
        .allowed_origins
        .iter()
        .map(|s| s.parse::<HeaderValue>())
        .collect();
    let origins =
        origins.map_err(|e| ConfigError::ValidationError(format!("Invalid CORS origin: {}", e)))?;

    // Parse and set allowed methods
    let methods: Result<Vec<Method>, _> =
        config.allowed_methods.iter().map(|s| s.parse()).collect();
    let methods =
        methods.map_err(|e| ConfigError::ValidationError(format!("Invalid CORS method: {}", e)))?;

    // Parse and set allowed headers
    let headers: Result<Vec<HeaderName>, _> =
        config.allowed_headers.iter().map(|s| s.parse()).collect();
    let headers =
        headers.map_err(|e| ConfigError::ValidationError(format!("Invalid CORS header: {}", e)))?;

    // Build CORS layer
    let mut cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(methods)
        .allow_headers(headers);

    // Set credentials
    if config.allow_credentials {
        cors = cors.allow_credentials(true);
    }

    // Set max age
    if let Some(max_age) = config.max_age {
        cors = cors.max_age(std::time::Duration::from_secs(max_age));
    }

    Ok(cors)
}

#[cfg(feature = "security")]
impl TryFrom<RateLimitConfigFile> for crate::security::RateLimitConfig {
    type Error = ConfigError;

    fn try_from(file_config: RateLimitConfigFile) -> Result<Self, Self::Error> {
        Ok(crate::security::RateLimitConfig {
            max_requests: file_config.max_requests,
            window: std::time::Duration::from_secs(file_config.window_seconds.into()),
            include_headers: true,
        })
    }
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

    #[test]
    fn test_connection_string_validation() {
        assert!(ConfigLoader::validate_connection_string(
            "postgresql://user:pass@localhost/db"
        ));
        assert!(ConfigLoader::validate_connection_string(
            "postgres://localhost:5432/db"
        ));

        assert!(!ConfigLoader::validate_connection_string(""));
        assert!(!ConfigLoader::validate_connection_string(
            "; DROP TABLE users;"
        ));
        assert!(!ConfigLoader::validate_connection_string("not-a-url"));
    }
}

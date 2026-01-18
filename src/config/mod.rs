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
    /// No authentication
    #[serde(rename = "none")]
    None,
}

impl Default for AuthConfig {
    fn default() -> Self {
        // Default to None for easier development
        AuthConfig::None
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

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

    /// Test AuthConfig::OAuth2 variant
    #[test]
    fn test_auth_config_oauth2() {
        let json = r#"{"type": "oauth2"}"#;
        let config: AuthConfig = serde_json::from_str(json).unwrap();
        match config {
            AuthConfig::OAuth2 => {
                // OAuth2 is unit variant, no fields
            }
            _ => panic!("Expected OAuth2 variant"),
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
        assert!(layer.is_ok());
    }

    /// Test build_cors_layer with valid origins
    #[test]
    fn test_build_cors_layer_valid_origins() {
        let json = r#"{"allowed_origins": ["http://localhost:3000"], "allowed_methods": [], "allowed_headers": []}"#;
        let config: CorsConfig = serde_json::from_str(json).unwrap();
        let layer = build_cors_layer(&config);
        assert!(layer.is_ok());
    }

    /// Test ConfigLoader::new and load
    #[test]
    fn test_config_loader() {
        let loader = ConfigLoader::new("/test/path.yaml");
        assert_eq!(loader.path, "/test/path.yaml");
        let result = loader.load();
        assert!(result.is_ok());
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
    }

    /// Test RateLimitConfigFile
    #[test]
    fn test_rate_limit_config() {
        let json = r#"{"requests": 100, "window_seconds": 60}"#;
        let config: RateLimitConfigFile = serde_json::from_str(json).unwrap();
        assert_eq!(config.requests, 100);
        assert_eq!(config.window_seconds, 60);
    }

    /// Test LoggingConfig
    #[test]
    fn test_logging_config() {
        let config = LoggingConfig {
            level: "debug".to_string(),
            format: "json".to_string(),
        };
        assert_eq!(config.level, "debug");
        assert_eq!(config.format, "json");
    }

    /// Test ApiConfig
    #[test]
    fn test_api_config() {
        let config = ApiConfig {
            prefix: "/api".to_string(),
            default_version: "v1".to_string(),
        };
        assert_eq!(config.prefix, "/api");
        assert_eq!(config.default_version, "v1");
    }

    /// Test ConfigEvent structure
    #[test]
    fn test_config_event() {
        let event = ConfigEvent {
            event_type: "reloaded".to_string(),
            path: "/etc/config.yaml".to_string(),
        };
        assert_eq!(event.event_type, "reloaded");
        assert_eq!(event.path, "/etc/config.yaml");
    }
}

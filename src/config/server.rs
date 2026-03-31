// Copyright (c) 2026 Kirky.X
//! Server configuration module
//!
//! This module provides server-related configuration types.

use crate::config::Config;
use serde::{Deserialize, Serialize};

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

impl ServerConfig {
    /// Validate server configuration
    pub fn validate(&self) -> Result<(), crate::config::ConfigError> {
        // Validate port range
        if self.port == 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "Server port cannot be 0".into(),
            ));
        }

        // Validate timeout is reasonable
        if self.request_timeout_secs == 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "Server request_timeout_secs cannot be 0".into(),
            ));
        }

        if self.request_timeout_secs > 86400 {
            return Err(crate::config::ConfigError::ValidationError(
                "Server request_timeout_secs should not exceed 86400 seconds (24 hours)".into(),
            ));
        }

        // Validate CORS if present
        if let Some(ref cors) = self.cors {
            cors.validate()?;
        }

        Ok(())
    }
}

#[cfg(feature = "validation")]
impl crate::config::ValidateConfig for ServerConfig {
    fn validate(&self) -> Result<(), crate::config::ConfigError> {
        use crate::config::ConfigError;
        
        // Validate port range
        if self.port == 0 {
            return Err(ConfigError::ValidationError(
                "Server port cannot be 0".into(),
            ));
        }

        // Validate timeout is reasonable
        if self.request_timeout_secs == 0 {
            return Err(ConfigError::ValidationError(
                "Server request_timeout_secs cannot be 0".into(),
            ));
        }

        if self.request_timeout_secs > 86400 {
            return Err(ConfigError::ValidationError(
                "Server request_timeout_secs should not exceed 86400 seconds (24 hours)".into(),
            ));
        }

        // Validate CORS if present
        if let Some(ref cors) = self.cors {
            cors.validate()?;
        }

        Ok(())
    }
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

// Re-export CorsConfig from cors module
pub use super::cors::CorsConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert!(config.host.is_empty()); // Default String is empty
        assert_eq!(config.port, 0); // Default u16 is 0
        assert_eq!(config.request_timeout_secs, 0); // Default u64 is 0
        assert!(config.cors.is_none());
    }

    #[test]
    fn test_server_config_validate_valid_port() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_server_config_validate_zero_port() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 0,
            request_timeout_secs: 30,
            cors: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_server_config_validate_zero_timeout() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 8080,
            request_timeout_secs: 0,
            cors: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_server_config_validate_excessive_timeout() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 8080,
            request_timeout_secs: 100000, // > 86400
            cors: None,
        };
        assert!(config.validate().is_err());
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
}

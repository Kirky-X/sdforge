// Copyright (c) 2026 Kirky.X
//! Application configuration
//!
//! This module contains the main application configuration structure
//! that combines all other configuration modules.

use serde::{Deserialize, Serialize};

use crate::config::{AuthConfig, ServerConfig, TimeoutConfig};
#[cfg(feature = "validation")]
use crate::config::ConfigError;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Authentication configuration
    #[serde(alias = "auth")]
    pub authentication: AuthConfig,
    /// Timeout configuration
    pub timeout: Option<TimeoutConfig>,
}

impl AppConfig {
    /// Create builder for configuration
    pub fn builder() -> AppConfigBuilder {
        AppConfigBuilder::default()
    }

    /// Validate configuration with cross-field validation
    #[cfg(feature = "validation")]
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate server configuration
        self.server.validate()?;

        // Validate authentication configuration
        self.authentication.validate()?;

        // Validate timeout configuration
        if let Some(ref timeout) = self.timeout {
            timeout.validate()?;
        }

        Ok(())
    }
}

#[cfg(feature = "validation")]
impl crate::config::ValidateConfig for AppConfig {
    fn validate(&self) -> Result<(), crate::config::ConfigError> {
        // Validate all sub-configs
        self.server.validate()?;
        self.authentication.validate()?;
        
        // Validate timeout if present
        if let Some(ref timeout) = self.timeout {
            timeout.validate()?;
        }

        // Cross-field validation
        // Example: Ensure rate limiting is configured if security is enabled
        // This can be expanded based on business requirements
        
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            authentication: AuthConfig::default(),
            timeout: Some(TimeoutConfig::default()),
        }
    }
}

/// Builder for AppConfig
#[derive(Default)]
pub struct AppConfigBuilder {
    server: Option<ServerConfig>,
    authentication: Option<AuthConfig>,
    timeout: Option<TimeoutConfig>,
}

impl AppConfigBuilder {
    /// Set server configuration
    pub fn server(mut self, server: ServerConfig) -> Self {
        self.server = Some(server);
        self
    }

    /// Set authentication configuration
    pub fn authentication(mut self, authentication: AuthConfig) -> Self {
        self.authentication = Some(authentication);
        self
    }

    /// Set timeout configuration
    pub fn timeout(mut self, timeout: TimeoutConfig) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build AppConfig with validation
    #[cfg(feature = "validation")]
    pub fn build(self) -> Result<AppConfig, crate::config::ConfigError> {
        let config = AppConfig {
            server: self.server.unwrap_or_default(),
            authentication: self.authentication.unwrap_or_default(),
            timeout: self.timeout,
        };
        
        // Validate the built configuration
        config.validate()?;
        
        Ok(config)
    }

    /// Build AppConfig without validation (legacy method)
    #[cfg(not(feature = "validation"))]
    pub fn build(self) -> AppConfig {
        AppConfig {
            server: self.server.unwrap_or_default(),
            authentication: self.authentication.unwrap_or_default(),
            timeout: self.timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        // ServerConfig uses derive(Config) which provides empty string for host and 0 for port
        assert_eq!(config.server.host, ""); // Default String is empty
        assert_eq!(config.server.port, 0); // Default u16 is 0
        
        // Verify other fields have proper defaults
        assert!(config.timeout.is_some());
        assert_eq!(config.timeout.as_ref().unwrap().default_timeout_secs, 30);
    }

    #[test]
    fn test_app_config_builder() {
        let result = AppConfig::builder()
            .server(ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 9000,
                request_timeout_secs: 30, // Must be > 0 for validation
                cors: None,
            })
            .build();
        
        // With validation feature, build() returns Result
        #[cfg(feature = "validation")]
        let config = result.expect("Failed to build config");
        
        #[cfg(not(feature = "validation"))]
        let config = result;

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9000);
    }
}

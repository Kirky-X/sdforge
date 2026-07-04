// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Application configuration
//!
//! This module contains the main application configuration structure
//! that combines all other configuration modules.

use serde::{Deserialize, Serialize};

use crate::config::ConfigError;
use crate::config::{AuthConfig, ServerConfig, TimeoutConfig};

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

impl crate::config::ValidateConfig for AppConfig {
    fn validate(&self) -> Result<(), crate::config::ConfigError> {
        // Delegate to inherent method to keep a single source of truth.
        // Previously this body was a near-duplicate of the inherent impl and
        // contained a YAGNI "Cross-field validation" placeholder comment.
        AppConfig::validate(self)
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
    ///
    /// BUG-3 修复: `timeout` 缺省时回退到 `TimeoutConfig::default()`，
    /// 与 `AppConfig::default()` 的行为保持一致。
    /// 原代码 `timeout: self.timeout` 在调用方未设置时产生 `None`，
    /// 而 `Default` 产生 `Some(TimeoutConfig::default())`，
    /// 导致两条构造路径语义不一致，下游 `if let Some(timeout)` 检查可能跳过验证。
    pub fn build(self) -> Result<AppConfig, crate::config::ConfigError> {
        let config = AppConfig {
            server: self.server.unwrap_or_default(),
            authentication: self.authentication.unwrap_or_default(),
            timeout: self.timeout.or_else(|| Some(TimeoutConfig::default())),
        };

        // Validate the built configuration
        config.validate()?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        // LOW-001: ServerConfig::default() 现在使用 fail-safe 常量
        assert_eq!(config.server.host, "127.0.0.1"); // fail-safe loopback
        assert_eq!(config.server.port, 8080);

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
        let config = result.expect("Failed to build config");

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9000);
    }
}

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
    /// Security configuration (headers, rate-limit, etc.)
    #[cfg(feature = "security")]
    pub security: crate::config::SecurityConfig,
    /// Cache configuration (capacity, TTL, etc.)
    #[cfg(feature = "cache")]
    pub cache: crate::config::CacheConfig,
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

        // Validate security configuration
        // UFCS 全限定调用：不依赖 trait 导入（避免与 use 清理互相冲突）
        #[cfg(feature = "security")]
        crate::config::ValidateConfig::validate(&self.security)?;

        // Validate cache configuration
        #[cfg(feature = "cache")]
        crate::config::ValidateConfig::validate(&self.cache)?;

        Ok(())
    }

    /// Build a `LimiteronAdapter` from the security rate-limit config (if any).
    ///
    /// Returns `Ok(Some(adapter))` when `security.rate_limit` is `Some`,
    /// `Ok(None)` when no rate-limit config is present, or `Err` when the
    /// limiteron config fails validation.
    #[cfg(feature = "ratelimit")]
    pub async fn build_rate_limiter(
        &self,
    ) -> Result<Option<crate::security::LimiteronAdapter>, crate::security::RateLimitError> {
        #[cfg(feature = "security")]
        {
            if let Some(ref rl_config) = self.security.rate_limit {
                let adapter = crate::security::LimiteronAdapter::builder()
                    .with_config(rl_config.clone())
                    .build()
                    .await?;
                return Ok(Some(adapter));
            }
        }
        Ok(None)
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
            #[cfg(feature = "security")]
            security: crate::config::SecurityConfig::default(),
            #[cfg(feature = "cache")]
            cache: crate::config::CacheConfig::default(),
        }
    }
}

/// Builder for AppConfig
#[derive(Default)]
pub struct AppConfigBuilder {
    server: Option<ServerConfig>,
    authentication: Option<AuthConfig>,
    timeout: Option<TimeoutConfig>,
    #[cfg(feature = "security")]
    security: Option<crate::config::SecurityConfig>,
    #[cfg(feature = "cache")]
    cache: Option<crate::config::CacheConfig>,
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

    /// Set security configuration
    #[cfg(feature = "security")]
    pub fn security(mut self, security: crate::config::SecurityConfig) -> Self {
        self.security = Some(security);
        self
    }

    /// Set cache configuration
    #[cfg(feature = "cache")]
    pub fn cache(mut self, cache: crate::config::CacheConfig) -> Self {
        self.cache = Some(cache);
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
            #[cfg(feature = "security")]
            security: self.security.unwrap_or_default(),
            #[cfg(feature = "cache")]
            cache: self.cache.unwrap_or_default(),
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
                ..Default::default()
            })
            .build();

        // With validation feature, build() returns Result
        let config = result.expect("Failed to build config");

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9000);
    }

    /// Cover the `ValidateConfig for AppConfig` trait impl (lines 50, 54)
    /// which delegates to the inherent `validate()` method. Existing tests
    /// only call the inherent method, leaving the trait impl body uncovered.
    #[test]
    fn test_validate_config_trait_for_app_config() {
        use crate::config::ValidateConfig;
        let config = AppConfig::default();
        assert!(ValidateConfig::validate(&config).is_ok());
    }
}

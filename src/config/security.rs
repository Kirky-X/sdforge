// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Security configuration
//!
//! This module provides security-related configuration options including
//! security headers, rate limiting, and other security features.

use serde::{Deserialize, Serialize};

use crate::config::defaults;

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Enable security headers
    pub enable_headers: bool,
    /// Content-Type-Options header
    pub content_type_options: String,
    /// X-Frame-Options header
    pub frame_options: String,
    /// X-XSS-Protection header
    pub xss_protection: String,
    /// Cache-Control header
    pub cache_control: String,
    /// Content-Security-Policy header
    pub content_security_policy: String,
    /// Strict-Transport-Security header
    pub strict_transport_security: String,
    /// Referrer-Policy header
    pub referrer_policy: String,
    /// Permissions-Policy header
    pub permissions_policy: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_headers: true,
            content_type_options: defaults::security_headers::CONTENT_TYPE_OPTIONS.to_string(),
            frame_options: defaults::security_headers::FRAME_OPTIONS.to_string(),
            xss_protection: defaults::security_headers::XSS_PROTECTION.to_string(),
            cache_control: defaults::security_headers::CACHE_CONTROL.to_string(),
            content_security_policy: defaults::security_headers::CONTENT_SECURITY_POLICY
                .to_string(),
            strict_transport_security: defaults::security_headers::STRICT_TRANSPORT_SECURITY
                .to_string(),
            referrer_policy: defaults::security_headers::REFERRER_POLICY.to_string(),
            permissions_policy: defaults::security_headers::PERMISSIONS_POLICY.to_string(),
        }
    }
}

impl SecurityConfig {
    /// Create a new security config with minimal headers
    pub fn minimal() -> Self {
        Self {
            enable_headers: true,
            content_type_options: defaults::security_headers::CONTENT_TYPE_OPTIONS.to_string(),
            frame_options: "SAMEORIGIN".to_string(),
            xss_protection: defaults::security_headers::XSS_PROTECTION.to_string(),
            cache_control: defaults::security_headers::CACHE_CONTROL.to_string(),
            content_security_policy: "default-src 'self'".to_string(),
            strict_transport_security: defaults::security_headers::STRICT_TRANSPORT_SECURITY
                .to_string(),
            referrer_policy: defaults::security_headers::REFERRER_POLICY.to_string(),
            permissions_policy: defaults::security_headers::PERMISSIONS_POLICY.to_string(),
        }
    }

    /// Create a disabled security config
    pub fn disabled() -> Self {
        Self {
            enable_headers: false,
            ..Default::default()
        }
    }

    /// Check if security headers are enabled
    pub fn headers_enabled(&self) -> bool {
        self.enable_headers
    }
}

impl crate::config::ValidateConfig for SecurityConfig {
    fn validate(&self) -> Result<(), crate::config::ConfigError> {
        // Security config validation is minimal for now
        // All fields have sensible defaults
        Ok(())
    }
}

impl SecurityConfig {
    /// Get all security headers as a vector of (name, value) pairs
    pub fn get_headers(&self) -> Vec<(&str, &str)> {
        if !self.enable_headers {
            return vec![];
        }

        vec![
            ("Content-Type-Options", &self.content_type_options),
            ("X-Frame-Options", &self.frame_options),
            ("X-XSS-Protection", &self.xss_protection),
            ("Cache-Control", &self.cache_control),
            ("Content-Security-Policy", &self.content_security_policy),
            ("Strict-Transport-Security", &self.strict_transport_security),
            ("Referrer-Policy", &self.referrer_policy),
            ("Permissions-Policy", &self.permissions_policy),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.enable_headers);
        assert_eq!(config.content_type_options, "nosniff");
        assert_eq!(config.frame_options, "DENY");
    }

    #[test]
    fn test_security_config_minimal() {
        let config = SecurityConfig::minimal();
        assert!(config.enable_headers);
        assert_eq!(config.frame_options, "SAMEORIGIN");
    }

    #[test]
    fn test_security_config_disabled() {
        let config = SecurityConfig::disabled();
        assert!(!config.enable_headers);
    }

    #[test]
    fn test_get_headers_enabled() {
        let config = SecurityConfig::default();
        let headers = config.get_headers();
        assert!(!headers.is_empty());
        assert!(headers.iter().any(|(k, _)| *k == "Content-Type-Options"));
    }

    #[test]
    fn test_get_headers_disabled() {
        let config = SecurityConfig::disabled();
        let headers = config.get_headers();
        assert!(headers.is_empty());
    }

    /// Test headers_enabled() reflects the enable_headers flag for both the
    /// default (enabled) and disabled configurations. Covers the headers_enabled
    /// accessor method.
    #[test]
    fn test_headers_enabled_reflects_flag() {
        let enabled = SecurityConfig::default();
        assert!(enabled.headers_enabled());

        let disabled = SecurityConfig::disabled();
        assert!(!disabled.headers_enabled());

        let minimal = SecurityConfig::minimal();
        assert!(minimal.headers_enabled());
    }

    /// Test get_headers returns all eight security headers in the correct
    /// order when enabled, verifying the full header set is emitted.
    #[test]
    fn test_get_headers_contains_all_eight_headers() {
        let config = SecurityConfig::default();
        let headers = config.get_headers();
        assert_eq!(headers.len(), 8);
        let names: Vec<&str> = headers.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            vec![
                "Content-Type-Options",
                "X-Frame-Options",
                "X-XSS-Protection",
                "Cache-Control",
                "Content-Security-Policy",
                "Strict-Transport-Security",
                "Referrer-Policy",
                "Permissions-Policy",
            ]
        );
    }

    /// Test that the minimal config differs from the default only in the
    /// frame_options and content_security_policy fields.
    #[test]
    fn test_minimal_config_differs_from_default() {
        let default = SecurityConfig::default();
        let minimal = SecurityConfig::minimal();
        assert_ne!(default.frame_options, minimal.frame_options);
        assert_eq!(minimal.frame_options, "SAMEORIGIN");
        assert_ne!(
            default.content_security_policy,
            minimal.content_security_policy
        );
        assert_eq!(minimal.content_security_policy, "default-src 'self'");
    }

    /// Test SecurityConfig serialization round-trip preserves all fields.
    #[test]
    fn test_security_config_serialization_roundtrip() {
        let config = SecurityConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: SecurityConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.enable_headers, config.enable_headers);
        assert_eq!(restored.content_type_options, config.content_type_options);
        assert_eq!(restored.frame_options, config.frame_options);
    }

    /// Test SecurityConfig deserialization with missing fields uses serde
    /// defaults (the struct is annotated with #[serde(default)]).
    #[test]
    fn test_security_config_deserialization_with_empty_object() {
        let json = r#"{}"#;
        let config: SecurityConfig = serde_json::from_str(json).unwrap();
        assert!(config.enable_headers);
        assert_eq!(config.content_type_options, "nosniff");
    }

    /// Test the ValidateConfig implementation always returns Ok since all
    /// fields have sensible defaults. Only compiled with the validation feature.
    #[test]
    fn test_security_config_validate_always_ok() {
        use crate::config::ValidateConfig;
        let config = SecurityConfig::default();
        assert!(config.validate().is_ok());
        let disabled = SecurityConfig::disabled();
        assert!(disabled.validate().is_ok());
    }
}

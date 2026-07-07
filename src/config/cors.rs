// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! CORS configuration module
//!
//! This module provides CORS-related configuration types and functions.

use serde::{Deserialize, Serialize};

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: Vec<String>,
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    pub allowed_headers: Vec<String>,
}

impl CorsConfig {
    /// Validate CORS configuration
    pub fn validate(&self) -> Result<(), crate::config::ConfigError> {
        // Check if allowed_origins is empty
        if self.allowed_origins.is_empty() {
            return Err(crate::config::ConfigError::ValidationError(
                "CORS allowed_origins cannot be empty".into(),
            ));
        }

        // Validate origin format: 必须含 scheme + host（"http://" alone 不合法）
        for origin in &self.allowed_origins {
            if !origin.starts_with("http://") && !origin.starts_with("https://") {
                return Err(crate::config::ConfigError::ValidationError(format!(
                    "Invalid CORS origin: {}. Must start with http:// or https://",
                    origin
                )));
            }
            // 检查 host 部分非空，与 build_cors_layer 行为一致（MED-004）
            let after_scheme = origin.split("://").nth(1).unwrap_or("");
            if after_scheme.is_empty() {
                return Err(crate::config::ConfigError::ValidationError(format!(
                    "Invalid CORS origin: {}. Must include host (e.g. http://example.com)",
                    origin
                )));
            }
        }

        Ok(())
    }
}

/// Build CORS layer from configuration
pub fn build_cors_layer(
    config: &CorsConfig,
) -> Result<tower_http::cors::CorsLayer, crate::config::ConfigError> {
    use tower_http::cors::{Any, CorsLayer};

    // Security: Validate that allowed_origins is not empty
    if config.allowed_origins.is_empty() {
        return Err(crate::config::ConfigError::ValidationError(
            "CORS allowed_origins cannot be empty. Use explicit origin list or disable CORS".into(),
        ));
    }

    // Validate origin format: 与 CorsConfig::validate() 保持一致
    for origin in &config.allowed_origins {
        if !origin.starts_with("http://") && !origin.starts_with("https://") {
            return Err(crate::config::ConfigError::ValidationError(format!(
                "Invalid CORS origin: {}. Must start with http:// or https://",
                origin
            )));
        }
        let after_scheme = origin.split("://").nth(1).unwrap_or("");
        if after_scheme.is_empty() {
            return Err(crate::config::ConfigError::ValidationError(format!(
                "Invalid CORS origin: {}. Must include host (e.g. http://example.com)",
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
        return Err(crate::config::ConfigError::ValidationError(
            "No valid origins found in CORS configuration".into(),
        ));
    }

    // Security: Never use Any as origin, always use explicit list
    let cors = cors.allow_origin(origins);

    Ok(cors)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_cors_config_validate_empty_origins() {
        let config = CorsConfig {
            allowed_origins: vec![],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec![],
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_cors_config_validate_invalid_origin_no_scheme() {
        let config = CorsConfig {
            allowed_origins: vec!["localhost:3000".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec![],
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid CORS origin"));
    }

    #[test]
    fn test_cors_config_validate_invalid_origin_http_only() {
        // MED-004: "http://" 仅含 scheme 无 host，应被拒绝（与 build_cors_layer 一致）
        let config = CorsConfig {
            allowed_origins: vec!["http://".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec![],
        };
        let result = config.validate();
        assert!(
            result.is_err(),
            "origin without host should be rejected, got: {:?}",
            result
        );
    }

    #[test]
    fn test_cors_config_validate_valid_origins() {
        let config = CorsConfig {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "https://example.com".to_string(),
            ],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_cors_config_clone() {
        let config = CorsConfig {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["Authorization".to_string()],
        };
        let cloned = config.clone();
        assert_eq!(cloned.allowed_origins, config.allowed_origins);
        assert_eq!(cloned.allowed_methods, config.allowed_methods);
    }

    #[test]
    fn test_build_cors_layer_invalid_origin_format() {
        let config = CorsConfig {
            allowed_origins: vec!["localhost:3000".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec![],
        };
        let result = build_cors_layer(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid CORS origin"));
    }

    #[test]
    fn test_build_cors_layer_no_valid_origins() {
        // Origin starts with http:// (passes format check) but contains a
        // newline (0x0A) which HeaderValue rejects (< 0x20 and != 0x09 HTAB),
        // resulting in an empty origins list after filter_map.
        let config = CorsConfig {
            allowed_origins: vec!["http://\ninvalid".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec![],
        };
        let result = build_cors_layer(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No valid origins"));
    }

    /// Cover the `after_scheme.is_empty()` branch in `build_cors_layer`
    /// (line 75) — origin with scheme but no host (e.g. "http://").
    /// Existing tests cover this branch in `CorsConfig::validate()` but not
    /// in `build_cors_layer`.
    #[test]
    fn test_build_cors_layer_empty_host_rejected() {
        let config = CorsConfig {
            allowed_origins: vec!["http://".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec![],
        };
        let result = build_cors_layer(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Must include host"));
    }
}

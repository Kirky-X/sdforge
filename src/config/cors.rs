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
            // 检查 host 部分非空，与 build_cors_layer 行为一致
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

/// 解析 `allowed_methods` 配置为 [`tower_http::cors::AllowMethods`]。
///
/// - 空列表 → `Any`（向后兼容：修复前实现硬编码 Any）
/// - 包含 `*` 或 `any`（大小写不敏感）→ `Any`
/// - 其余按 HTTP method 名精确解析，非法条目报错（fail-closed）
///
/// HIGH 修复：此前 `allowed_methods` 配置被完全忽略，`build_cors_layer`
/// 硬编码 `.allow_methods(Any)`。
fn parse_allowed_methods(
    methods: &[String],
) -> Result<tower_http::cors::AllowMethods, crate::config::ConfigError> {
    use tower_http::cors::Any;
    if methods.is_empty()
        || methods
            .iter()
            .any(|m| m == "*" || m.eq_ignore_ascii_case("any"))
    {
        return Ok(Any.into());
    }
    let mut parsed = Vec::with_capacity(methods.len());
    for m in methods {
        let method = axum::http::Method::from_bytes(m.as_bytes()).map_err(|_| {
            crate::config::ConfigError::ValidationError(format!(
                "Invalid CORS method: {}. Use standard HTTP method names (e.g. GET, POST) or \"*\".",
                m
            ))
        })?;
        parsed.push(method);
    }
    Ok(tower_http::cors::AllowMethods::list(parsed))
}

/// 解析 `allowed_headers` 配置为 [`tower_http::cors::AllowHeaders`]。
///
/// 规则与 [`parse_allowed_methods`] 一致（头部名大小写不敏感，内部归一化
/// 为小写）。
fn parse_allowed_headers(
    headers: &[String],
) -> Result<tower_http::cors::AllowHeaders, crate::config::ConfigError> {
    use tower_http::cors::Any;
    if headers.is_empty() || headers.iter().any(|h| h == "*") {
        return Ok(Any.into());
    }
    let mut parsed = Vec::with_capacity(headers.len());
    for h in headers {
        let name =
            axum::http::HeaderName::from_bytes(h.to_lowercase().as_bytes()).map_err(|_| {
                crate::config::ConfigError::ValidationError(format!(
                    "Invalid CORS header: {}. Use valid header names (e.g. Content-Type) or \"*\".",
                    h
                ))
            })?;
        parsed.push(name);
    }
    Ok(tower_http::cors::AllowHeaders::list(parsed))
}

/// Build CORS layer from configuration
///
/// HIGH 修复：`allowed_methods` / `allowed_headers` 配置现已生效。
/// 空列表保持修复前的宽松行为（Any）；显式列表精确生效。
pub fn build_cors_layer(
    config: &CorsConfig,
) -> Result<tower_http::cors::CorsLayer, crate::config::ConfigError> {
    use tower_http::cors::CorsLayer;

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

    let cors = CorsLayer::new()
        .allow_methods(parse_allowed_methods(&config.allowed_methods)?)
        .allow_headers(parse_allowed_headers(&config.allowed_headers)?);

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
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid CORS origin")
        );
    }

    #[test]
    fn test_cors_config_validate_invalid_origin_http_only() {
        // "http://" 仅含 scheme 无 host，应被拒绝（与 build_cors_layer 一致）
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
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid CORS origin")
        );
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
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Must include host")
        );
    }

    /// HIGH 修复回归：CORS 配置解析 fail-closed——非法头部名报错；
    /// 合法 method 列表与通配符均正确构建。（行为级验证见
    /// 下方 `cors_behavior_tests`：预检请求按允许列表放行/拒绝）
    #[test]
    fn test_build_cors_layer_methods_config_enforced() {
        // 非法头部名 → 报错
        let config = CorsConfig {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allowed_methods: vec![],
            allowed_headers: vec!["Content Type".to_string()],
        };
        let result = build_cors_layer(&config);
        assert!(result.is_err(), "invalid header must fail closed");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid CORS header")
        );

        // 合法 method 列表 → 构建成功
        let config = CorsConfig {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
        };
        assert!(build_cors_layer(&config).is_ok());

        // 通配符语义保留
        let config = CorsConfig {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allowed_methods: vec!["*".to_string()],
            allowed_headers: vec!["*".to_string()],
        };
        assert!(build_cors_layer(&config).is_ok());
    }

    /// HIGH 修复回归：parse_allowed_headers 对非法头部名 fail-closed。
    #[test]
    fn test_parse_allowed_headers_rejects_invalid_name() {
        let headers = vec!["Content Type".to_string()]; // 空格非法
        let result = parse_allowed_headers(&headers);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid CORS header")
        );
    }
}

/// 行为级回归（HIGH 修复）：CORS method 配置真实生效。
/// 修复前 build_cors_layer 硬编码 `.allow_methods(Any)`，任何预检都放行。
#[cfg(all(test, feature = "http", feature = "tokio"))]
mod cors_behavior_tests {
    use super::*;
    use axum::http::{Method, Request};
    use tower::ServiceExt;

    fn cors_router(config: &CorsConfig) -> axum::Router {
        use axum::Router;
        use axum::routing::get;
        let layer = build_cors_layer(config).expect("valid cors config");
        Router::new()
            .route("/x", get(|| async { "ok" }))
            .layer(layer)
    }

    fn preflight(method: &str) -> Request<axum::body::Body> {
        Request::builder()
            .method(Method::OPTIONS)
            .uri("/x")
            .header("Origin", "http://localhost:3000")
            .header("Access-Control-Request-Method", method)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn preflight_rejects_method_outside_allow_list() {
        let config = CorsConfig {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec![],
        };
        let app = cors_router(&config);
        let resp = app.oneshot(preflight("DELETE")).await.unwrap();
        // tower-http 0.7 不在服务端拒绝预检，而是回显 Access-Control-Allow-Methods
        // 交给浏览器执行。因此断言该头精确反映配置（修复前硬编码 Any →
        // 该头不存在，浏览器回退为放行所有 method）。
        let allow_methods = resp
            .headers()
            .get("access-control-allow-methods")
            .expect("allow-methods header must reflect explicit config")
            .to_str()
            .unwrap();
        let echoed: Vec<&str> = allow_methods.split(',').map(str::trim).collect();
        assert_eq!(
            echoed,
            vec!["GET", "POST"],
            "config methods must be echoed exactly, got: {}",
            allow_methods
        );
    }

    #[tokio::test]
    async fn preflight_allows_method_in_allow_list() {
        let config = CorsConfig {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec![],
        };
        let app = cors_router(&config);
        let resp = app.oneshot(preflight("POST")).await.unwrap();
        assert!(
            resp.headers().get("access-control-allow-origin").is_some(),
            "allowed preflight must carry allow-origin header"
        );
    }
}

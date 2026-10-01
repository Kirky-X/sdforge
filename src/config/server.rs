// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Server configuration module
//!
//! This module provides server-related configuration types.

use serde::{Deserialize, Serialize};

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Maximum request body size in bytes (default 10 MiB)
    ///
    /// HIGH 修复：此前 10MB 上限硬编码在 `http::build_with_config` 中，
    /// 配置结构无对应字段，运维无法调整上传上限。
    pub max_body_size: usize,
    /// CORS configuration
    pub cors: Option<CorsConfig>,
    /// TLS configuration（消费方：`http::tls`，feature = `serve-tls`；纯数据
    /// 配置，任何特性组合下均可声明与校验。与 gRPC 侧 `GrpcServerConfig::tls`
    /// 的 cfg 门控不同——那边类型绑定 tonic 必须门控，这边零 TLS 依赖故恒在）。
    pub tls: Option<TlsConfig>,
    /// Idempotency replay protection (feature = `idempotency`).
    ///
    /// `enabled` 默认 false —— 开启后仅携带 `Idempotency-Key` 头的
    /// POST/PUT/PATCH 请求参与重放防护，其余请求零开销透行。
    #[cfg(feature = "idempotency")]
    pub idempotency: IdempotencyConfig,
}

/// Idempotency replay-protection configuration (feature = `idempotency`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "idempotency")]
pub struct IdempotencyConfig {
    /// 是否启用幂等中间件（默认 false）。
    pub enabled: bool,
    /// 完成响应的重放窗口（秒，默认 86400 = 24h）。
    pub ttl_secs: i64,
    /// 在途 claim 的阻塞上限（秒，默认 30）——handler 崩溃后同 key 重试
    /// 需等待该窗口；此前硬编码 30（复查 配置化）。
    #[serde(default)]
    pub inflight_ttl_secs: i64,
    /// 超过该大小的响应不缓存（默认 1 MiB）。
    pub max_response_bytes: usize,
    /// 外部注入的 store（测试/多路由共享复用）。None = 内部新建。
    #[serde(skip, default)]
    pub store: Option<std::sync::Arc<crate::cache::IdempotencyStore>>,
}

#[cfg(feature = "idempotency")]
impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl_secs: 86_400,
            inflight_ttl_secs: 30,
            max_response_bytes: 1024 * 1024,
            store: None,
        }
    }
}

#[cfg(feature = "idempotency")]
impl IdempotencyConfig {
    /// Validate idempotency configuration.
    pub fn validate(&self) -> Result<(), crate::config::ConfigError> {
        if self.enabled && self.ttl_secs <= 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "Idempotency ttl_secs must be positive when enabled".into(),
            ));
        }
        if self.enabled && self.inflight_ttl_secs <= 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "Idempotency ttl_secs must be positive when enabled".into(),
            ));
        }
        if self.enabled && self.max_response_bytes == 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "Idempotency max_response_bytes cannot be 0 when enabled".into(),
            ));
        }
        Ok(())
    }
}

impl Default for ServerConfig {
    /// Fail-safe 默认值：loopback host + 合理 port/timeout
    /// (避免 derive(Default) 产生空 host/port=0 的无效配置)
    fn default() -> Self {
        use crate::config::{DEFAULT_HOST, DEFAULT_PORT, DEFAULT_REQUEST_TIMEOUT_SECS};
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
            max_body_size: 10 * 1024 * 1024, // 10 MiB
            cors: None,
            tls: None,
            #[cfg(feature = "idempotency")]
            idempotency: crate::config::IdempotencyConfig::default(),
        }
    }
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

        // Body limit must be positive: 0 would make RequestBodyLimitLayer
        // reject every request with a body.
        if self.max_body_size == 0 {
            return Err(crate::config::ConfigError::ValidationError(
                "Server max_body_size cannot be 0".into(),
            ));
        }

        // Validate CORS if present
        if let Some(ref cors) = self.cors {
            cors.validate()?;
        }

        if let Some(ref tls) = self.tls {
            tls.validate()?;
        }

        #[cfg(feature = "idempotency")]
        self.idempotency.validate()?;

        Ok(())
    }
}

impl crate::config::ValidateConfig for ServerConfig {
    fn validate(&self) -> Result<(), crate::config::ConfigError> {
        // Delegate to inherent method to keep a single source of truth.
        // Previously this body was a verbatim duplicate of the inherent impl.
        ServerConfig::validate(self)
    }
}

/// ALPN 协议默认列表：HTTP/2 优先、HTTP/1.1 兜底（覆盖现代与旧客户端）。
fn default_alpn_protocols() -> Vec<String> {
    vec!["h2".to_string(), "http/1.1".to_string()]
}

/// TLS configuration
///
/// 纯数据配置：证书/密钥的加载与 TLS 终止由 `http::tls`（feature =
/// `serve-tls`，rustls）完成；gRPC 侧的对应物是 `grpc::GrpcServerConfig::tls`
/// （feature = `grpc-tls`，tonic `ServerTlsConfig` 接线）——两者文档互链、
/// 实现独立。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to certificate file (PEM)
    cert_path: String,
    /// Path to private key file (PEM)
    key_path: String,
    /// 向客户端宣告的 ALPN 协议列表（TLS wire 每项 ≤ 255 字节且非空）。
    /// 空 = 不宣告 ALPN。缺省 `["h2", "http/1.1"]`。
    #[serde(default = "default_alpn_protocols")]
    alpn_protocols: Vec<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: String::new(),
            key_path: String::new(),
            alpn_protocols: default_alpn_protocols(),
        }
    }
}

impl TlsConfig {
    /// Create a TLS configuration from PEM file paths.
    pub fn new(cert_path: impl Into<String>, key_path: impl Into<String>) -> Self {
        Self {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            alpn_protocols: default_alpn_protocols(),
        }
    }

    /// Get certificate path
    pub fn cert_path(&self) -> &str {
        &self.cert_path
    }

    /// Get private key path
    pub fn key_path(&self) -> &str {
        &self.key_path
    }

    /// Get the ALPN protocol list offered to clients.
    pub fn alpn_protocols(&self) -> &[String] {
        &self.alpn_protocols
    }

    /// Override the ALPN protocol list (builder style). Empty slice = 不宣告。
    pub fn with_alpn_protocols(mut self, protocols: Vec<String>) -> Self {
        self.alpn_protocols = protocols;
        self
    }

    /// Validate TLS configuration.
    ///
    /// 证书/密钥路径必须非空；ALPN 每项必须非空且 ≤ 255 字节
    /// （TLS wire 格式为单字节长度前缀 + 内容，超限握手期才会暴露，
    /// 提前到配置校验阶段拒绝）。
    pub fn validate(&self) -> Result<(), crate::config::ConfigError> {
        if self.cert_path.is_empty() {
            return Err(crate::config::ConfigError::ValidationError(
                "TLS cert_path cannot be empty".into(),
            ));
        }
        if self.key_path.is_empty() {
            return Err(crate::config::ConfigError::ValidationError(
                "TLS key_path cannot be empty".into(),
            ));
        }
        for protocol in &self.alpn_protocols {
            if protocol.is_empty() {
                return Err(crate::config::ConfigError::ValidationError(
                    "TLS alpn_protocols entries cannot be empty".into(),
                ));
            }
            if protocol.len() > 255 {
                return Err(crate::config::ConfigError::ValidationError(
                    "TLS alpn_protocols entries cannot exceed 255 bytes".into(),
                ));
            }
        }
        Ok(())
    }
}

// Re-export CorsConfig from cors module
pub use super::cors::CorsConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        // Default 现在使用 fail-safe 常量（loopback + 合理端口/超时）
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1"); // fail-safe loopback
        assert_eq!(config.port, 8080);
        assert_eq!(config.request_timeout_secs, 30);
        assert!(config.cors.is_none());
    }

    #[test]
    fn test_server_config_validate_valid_port() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
        };
        assert!(config.cors.is_some());
        let cors = config.cors.unwrap();
        assert_eq!(cors.allowed_origins.len(), 1);
    }

    #[test]
    fn test_tls_config_getters() {
        let config = TlsConfig::new("/etc/ssl/cert.pem", "/etc/ssl/key.pem");
        assert_eq!(config.cert_path(), "/etc/ssl/cert.pem");
        assert_eq!(config.key_path(), "/etc/ssl/key.pem");
        // 默认 ALPN：h2 优先、http/1.1 兜底
        assert_eq!(
            config.alpn_protocols(),
            &["h2".to_string(), "http/1.1".to_string()]
        );
    }

    #[test]
    fn test_tls_config_serialization() {
        let config = TlsConfig::new("/path/to/cert.pem", "/path/to/key.pem");
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: TlsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cert_path(), "/path/to/cert.pem");
        assert_eq!(deserialized.key_path(), "/path/to/key.pem");
    }

    #[test]
    fn test_tls_config_deserialization_defaults_alpn() {
        // 旧格式配置（无 alpn 字段）反序列化时补默认 ALPN 列表
        let json = r#"{"cert_path": "/c.pem", "key_path": "/k.pem"}"#;
        let config: TlsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.alpn_protocols(),
            &["h2".to_string(), "http/1.1".to_string()]
        );
    }

    #[test]
    fn test_tls_config_custom_alpn_roundtrip() {
        let config =
            TlsConfig::new("/c.pem", "/k.pem").with_alpn_protocols(vec!["http/1.1".into()]);
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: TlsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.alpn_protocols(), &["http/1.1".to_string()]);
    }

    #[test]
    fn test_tls_config_validate_rejects_empty_paths() {
        let config = TlsConfig::default();
        let err = config.validate().unwrap_err().to_string();
        assert!(
            err.contains("cert_path"),
            "empty cert_path must be named: {err}"
        );

        let config = TlsConfig::new("/c.pem", "");
        let err = config.validate().unwrap_err().to_string();
        assert!(
            err.contains("key_path"),
            "empty key_path must be named: {err}"
        );
    }

    #[test]
    fn test_tls_config_validate_rejects_invalid_alpn_entries() {
        let empty = TlsConfig::new("/c.pem", "/k.pem").with_alpn_protocols(vec![String::new()]);
        assert!(
            empty.validate().is_err(),
            "empty ALPN entry must be rejected"
        );

        let oversized =
            TlsConfig::new("/c.pem", "/k.pem").with_alpn_protocols(vec!["x".repeat(256)]);
        assert!(
            oversized.validate().is_err(),
            "ALPN entry over 255 bytes must be rejected"
        );

        let ok = TlsConfig::new("/c.pem", "/k.pem").with_alpn_protocols(vec!["h2".into()]);
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn test_server_config_validate_propagates_tls_error() {
        let config = ServerConfig {
            tls: Some(TlsConfig::default()),
            ..Default::default()
        };
        let err = config.validate().unwrap_err().to_string();
        assert!(
            err.contains("cert_path"),
            "server validate must propagate TLS validation error: {err}"
        );
    }

    // ============================================================================
    // validate() with Some(cors) branch coverage
    //
    // The `if let Some(ref cors) = self.cors { cors.validate()?; }` branch in
    // ServerConfig::validate() was previously uncovered: test_server_config_with_cors
    // constructs a config with CORS but never calls validate(), while the other
    // validate tests use cors: None.
    // ============================================================================

    /// Test ServerConfig::validate() succeeds when a valid CorsConfig is
    /// present. Covers the `if let Some(ref cors)` branch with a passing
    /// cors.validate() call.
    #[test]
    fn test_server_config_validate_with_valid_cors() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: Some(CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
                allowed_methods: vec!["GET".to_string()],
                allowed_headers: vec!["Authorization".to_string()],
            }),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    /// Test ServerConfig::validate() propagates the error when the CorsConfig
    /// is invalid (empty allowed_origins). Covers the `cors.validate()?` error
    /// propagation path via the `?` operator.
    #[test]
    fn test_server_config_validate_propagates_cors_error() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: Some(CorsConfig {
                allowed_origins: vec![],
                allowed_methods: vec!["GET".to_string()],
                allowed_headers: vec![],
            }),
            ..Default::default()
        };
        let result = config.validate();
        assert!(
            result.is_err(),
            "Invalid CORS should propagate validation error"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("empty"),
            "Error should mention empty origins: {}",
            err_msg
        );
    }

    /// Test ServerConfig::validate() propagates the error when the CorsConfig
    /// has an invalid origin format (missing scheme). Covers the
    /// `cors.validate()?` error propagation path for invalid origin format.
    #[test]
    fn test_server_config_validate_propagates_invalid_origin_error() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: Some(CorsConfig {
                allowed_origins: vec!["localhost:3000".to_string()],
                allowed_methods: vec!["GET".to_string()],
                allowed_headers: vec![],
            }),
            ..Default::default()
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

    /// Test ServerConfig with boundary timeout value (exactly 86400) passes
    /// validation. Covers the `>` boundary of the timeout check.
    #[test]
    fn test_server_config_validate_boundary_timeout_86400() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 8080,
            request_timeout_secs: 86400,
            cors: None,
            ..Default::default()
        };
        assert!(
            config.validate().is_ok(),
            "86400 seconds should be allowed (boundary)"
        );
    }

    /// Test ServerConfig deserialization with serde(default) fills missing
    /// fields. Covers the `#[serde(default)]` attribute behavior.
    #[test]
    fn test_server_config_deserialization_with_default_fields() {
        let json = r#"{"host": "0.0.0.0", "port": 3000}"#;
        let config: ServerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        // #[serde(default)] 现在使用 Default trait，request_timeout_secs 默认 30
        assert_eq!(
            config.request_timeout_secs, 30,
            "Missing field should use Default (30 secs, fail-safe)"
        );
        assert!(config.cors.is_none(), "Missing cors should default to None");
    }

    /// Cover the `ValidateConfig for ServerConfig` trait impl (lines 70, 73)
    /// which delegates to the inherent `validate()` method. Existing tests
    /// only call the inherent method, leaving the trait impl body uncovered.
    #[test]
    fn test_validate_config_trait_for_server_config() {
        use crate::config::ValidateConfig;
        let config = ServerConfig::default();
        assert!(ValidateConfig::validate(&config).is_ok());
    }
}

// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 应用配置示例
//!
//! 本示例展示 SDForge 配置管理的三种使用模式：
//!
//! 1. **开箱即用** — `AppConfig::default()`
//! 2. **Builder 模式** — `AppConfig::builder().server(...).build()`
//! 3. **序列化/反序列化** — 通过 serde 持久化配置
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --features http_examples --example config/app_config
//! ```

use sdforge::config::{
    ApiConfig, ApiKeySeed, AppConfig, AuthConfig, EnvHelper, ServerConfig, TimeoutConfig,
    TracingConfig,
};

// =============================================================================
// 模式 1: 开箱即用 — 默认配置
// =============================================================================

/// 使用默认值创建应用配置。
///
/// `AppConfig::default()` 提供合理的默认值：
/// - server: `127.0.0.1:8080`, 30s 请求超时
/// - authentication: `AuthConfig::None`
/// - timeout: 默认 30s 超时 + 上传/导出路由特殊超时
pub fn default_config() -> AppConfig {
    AppConfig::default()
}

// =============================================================================
// 模式 2: Builder 模式 — 自定义配置
// =============================================================================

/// 使用 Builder 模式构建自定义配置。
///
/// Builder 允许部分定制 — 未指定的字段使用默认值。
pub fn build_custom_config() -> AppConfig {
    let server = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 3000,
        request_timeout_secs: 60,
        cors: None,
    };

    let auth = AuthConfig::ApiKey {
        header_name: "X-API-Key".to_string(),
        prefix: "sk_".to_string(),
        // HIGH-003 修复后：配置里声明种子键，避免空 key store 把 API 锁死在 401
        keys: vec![ApiKeySeed {
            key: "sk_demo_0123456789abcdef".to_string(),
            permissions: vec!["read".to_string()],
        }],
    };

    let timeout = TimeoutConfig {
        default_timeout_secs: 45,
        route_timeouts: std::collections::HashMap::new(),
    };

    AppConfig::builder()
        .server(server)
        .authentication(auth)
        .timeout(timeout)
        .build()
        .expect("config build failed")
}

// =============================================================================
// 模式 3: 序列化/反序列化
// =============================================================================

/// 将配置序列化为 JSON 字符串。
///
/// 可用于持久化配置或通过网络传输。
pub fn serialize_config(config: &AppConfig) -> Result<String, String> {
    serde_json::to_string_pretty(config).map_err(|e| e.to_string())
}

/// 从 JSON 字符串反序列化配置。
pub fn deserialize_config(json: &str) -> Result<AppConfig, String> {
    serde_json::from_str(json).map_err(|e| e.to_string())
}

// =============================================================================
// 子配置示例
// =============================================================================

/// 演示 `ServerConfig` 的构建和验证。
pub fn demo_server_config() -> ServerConfig {
    ServerConfig {
        host: "0.0.0.0".to_string(),
        port: 8080,
        request_timeout_secs: 30,
        cors: None,
    }
}

/// 演示 `AuthConfig` 的不同变体。
pub fn demo_auth_configs() -> Vec<AuthConfig> {
    vec![
        AuthConfig::None,
        AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "sk_".to_string(),
            keys: vec![],
        },
        AuthConfig::Jwt {
            secret: "a-very-long-and-secure-jwt-secret-key-32+chars!".to_string(),
        },
    ]
}

/// 演示 `TimeoutConfig` 的路由级超时。
pub fn demo_timeout_config() -> TimeoutConfig {
    let mut route_timeouts = std::collections::HashMap::new();
    route_timeouts.insert("/api/upload".to_string(), 120u64);
    route_timeouts.insert("/api/export".to_string(), 300u64);
    route_timeouts.insert("/api/health".to_string(), 5u64);

    TimeoutConfig {
        default_timeout_secs: 30,
        route_timeouts,
    }
}

/// 演示辅助配置类型。
pub fn demo_helper_configs() {
    let api_config = ApiConfig {
        prefix: "/api".to_string(),
        default_version: "v1".to_string(),
    };

    let tracing = TracingConfig { enabled: true };

    let env = EnvHelper {
        environment: "production".to_string(),
    };

    println!(
        "  ApiConfig: prefix={}, version={}",
        api_config.prefix, api_config.default_version
    );
    println!("  TracingConfig: enabled={}", tracing.enabled);
    println!("  EnvHelper: environment={}", env.environment);
}

// =============================================================================
// Main Entry Point
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚙️  SDForge Configuration Example");
    println!("==================================\n");

    // 模式 1: 默认配置
    println!("📦 模式 1: 默认配置 (AppConfig::default())");
    let default = default_config();
    println!(
        "  server: {}:{}, timeout: {}s",
        default.server.host, default.server.port, default.server.request_timeout_secs
    );
    println!("  auth: {:?}", default.authentication);
    println!();

    // 模式 2: Builder 模式
    println!("🔧 模式 2: Builder 模式 (自定义配置)");
    let custom = build_custom_config();
    println!(
        "  server: {}:{}, timeout: {}s",
        custom.server.host, custom.server.port, custom.server.request_timeout_secs
    );
    println!("  auth: {:?}", custom.authentication);
    println!();

    // 模式 3: 序列化/反序列化
    println!("💾 模式 3: 序列化/反序列化");
    let json = serialize_config(&custom)?;
    println!("  JSON (前 200 字符):");
    let preview = if json.len() > 200 {
        &json[..200]
    } else {
        &json
    };
    println!("  {}", preview);
    println!("  ...");

    let restored = deserialize_config(&json)?;
    println!(
        "  反序列化: server={}:{}, auth={:?}",
        restored.server.host, restored.server.port, restored.authentication
    );
    println!();

    // 子配置演示
    println!("📋 子配置示例:");
    let server = demo_server_config();
    println!(
        "  ServerConfig: {}:{}, timeout={}s",
        server.host, server.port, server.request_timeout_secs
    );

    println!();
    println!("  AuthConfig 变体:");
    for (i, auth) in demo_auth_configs().iter().enumerate() {
        println!("    [{}] {:?}", i, auth);
    }

    println!();
    println!("  TimeoutConfig 路由级超时:");
    let timeout = demo_timeout_config();
    println!("    default: {}s", timeout.default_timeout_secs);
    for (route, secs) in &timeout.route_timeouts {
        println!("    {}: {}s", route, secs);
    }
    println!(
        "    get_timeout(\"/api/upload\") = {}s",
        timeout.get_timeout("/api/upload")
    );
    println!(
        "    get_timeout(\"/api/unknown\") = {}s (回退到默认)",
        timeout.get_timeout("/api/unknown")
    );

    println!();
    println!("  辅助配置类型:");
    demo_helper_configs();

    println!();
    println!("✓ 配置示例完成");

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_server() {
        let config = default_config();
        // LOW-001: ServerConfig::default() 现在使用 fail-safe 常量（loopback + 8080）
        assert_eq!(
            config.server.host, "127.0.0.1",
            "default host is fail-safe loopback"
        );
        assert_eq!(config.server.port, 8080, "default port is 8080");
    }

    #[test]
    fn test_build_custom_config() {
        let config = build_custom_config();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.server.request_timeout_secs, 60);
        // AuthConfig::ApiKey variant check
        match &config.authentication {
            AuthConfig::ApiKey {
                header_name,
                prefix,
                ..
            } => {
                assert_eq!(header_name, "X-API-Key");
                assert_eq!(prefix, "sk_");
            }
            other => panic!("expected ApiKey auth, got {:?}", other),
        }
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let original = build_custom_config();
        let json = serialize_config(&original).expect("serialize should succeed");
        let restored = deserialize_config(&json).expect("deserialize should succeed");
        assert_eq!(restored.server.host, original.server.host);
        assert_eq!(restored.server.port, original.server.port);
    }

    #[test]
    fn test_demo_server_config() {
        let server = demo_server_config();
        assert_eq!(server.host, "0.0.0.0");
        assert_eq!(server.port, 8080);
    }

    #[test]
    fn test_demo_auth_configs_variants() {
        let configs = demo_auth_configs();
        assert_eq!(configs.len(), 3, "should have 3 auth config variants");
        // Verify all variants are present
        assert!(matches!(configs[0], AuthConfig::None));
        assert!(matches!(configs[1], AuthConfig::ApiKey { .. }));
        assert!(matches!(configs[2], AuthConfig::Jwt { .. }));
    }

    #[test]
    fn test_timeout_config_route_lookup() {
        let timeout = demo_timeout_config();
        // Route-specific timeout
        assert_eq!(timeout.get_timeout("/api/upload"), 120);
        assert_eq!(timeout.get_timeout("/api/export"), 300);
        // Unknown route falls back to default
        assert_eq!(timeout.get_timeout("/api/unknown"), 30);
    }

    #[test]
    fn test_deserialize_invalid_json_fails() {
        let result = deserialize_config("not valid json");
        assert!(result.is_err(), "invalid JSON should produce an error");
    }
}
// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # gRPC 服务端示例
//!
//! 本示例展示如何使用 SDForge 构建 gRPC 服务：
//!
//! 1. 创建 `GrpcRoute` 并通过 inventory 注册
//! 2. 配置 `GrpcServerConfig`（连接数、超时、可选 JWT 认证）
//! 3. 使用 `build_server` 或 `build_server_with_config` 启动服务
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --features grpc_examples --example grpc/server
//! ```
//!
//! ## 客户端调用
//!
//! 使用 grpcurl 或任意 gRPC 客户端调用 `SdForgeService`：
//!
//! ```bash
//! grpcurl -plaintext -d '{"method":"GET"}' localhost:50051 \
//!   sdforge.v1.SdForgeService/Call
//! ```

use sdforge::core::ApiMetadata;
use sdforge::grpc::{
    build_server, build_server_with_config, GrpcRoute, GrpcServerConfig, SdForgeGrpcService,
};

// =============================================================================
// 路由注册
// =============================================================================

/// 演示如何创建一个 gRPC 路由条目。
///
/// `GrpcRoute` 包含服务名和 API 元数据。在宏生成的代码中，这些通过
/// `inventory::submit!(GrpcRouteRegistration::new(...))` 在编译时注册。
/// 本示例展示手动创建的方式。
pub fn create_sample_route() -> GrpcRoute {
    GrpcRoute::new(
        "SdForgeService".to_string(),
        ApiMetadata::new(
            "sdforge_grpc".to_string(),
            "v1".to_string(),
            "SDForge gRPC service example".to_string(),
            None,
            false,
        ),
    )
}

// =============================================================================
// 服务器配置
// =============================================================================

/// 默认 gRPC 服务器配置。
///
/// `GrpcServerConfig` 控制：
/// - `max_connections` — 每连接并发流上限（tonic `concurrency_limit_per_connection`）
/// - `timeout_seconds` — 请求超时
/// - `auth` — 可选 JWT 认证拦截器（需 `security` feature）
pub fn default_server_config() -> GrpcServerConfig {
    GrpcServerConfig::default()
}

/// 自定义配置示例：限制并发、设置 60s 超时。
pub fn custom_server_config() -> GrpcServerConfig {
    GrpcServerConfig {
        max_connections: 200,
        timeout_seconds: 60,
        #[cfg(feature = "security_examples")]
        auth: None,
    }
}

// =============================================================================
// 默认服务
// =============================================================================

/// 默认 gRPC 服务实例。
///
/// `SdForgeGrpcService` 实现了 `SdForgeService` trait，
/// 提供 `Call` 和 `GetInfo` 两个 RPC 方法。
pub fn default_service() -> SdForgeGrpcService {
    SdForgeGrpcService::default()
}

// =============================================================================
// 服务器启动（演示）
// =============================================================================

/// 启动 gRPC 服务器（默认配置）。
///
/// 实际启动需要绑定到可用端口。本函数返回 future 供调用方决定何时运行。
///
/// # Errors
///
/// 当地址格式无效或 tonic 运行出错时返回错误。
pub async fn start_default(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    build_server(addr).await
}

/// 启动 gRPC 服务器（自定义配置）。
///
/// # Errors
///
/// 当地址格式无效或 tonic 运行出错时返回错误。
pub async fn start_with_config(
    addr: &str,
    config: GrpcServerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    build_server_with_config(addr, config).await
}

// =============================================================================
// Main Entry Point
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SDForge gRPC Example");
    println!("========================\n");

    // 演示路由创建
    let route = create_sample_route();
    println!("✓ GrpcRoute created: {:?}", route);

    // 演示默认配置
    let default_config = default_server_config();
    println!(
        "✓ Default GrpcServerConfig: max_connections={}, timeout={}s",
        default_config.max_connections, default_config.timeout_seconds
    );

    // 演示自定义配置
    let custom_config = custom_server_config();
    println!(
        "✓ Custom GrpcServerConfig: max_connections={}, timeout={}s",
        custom_config.max_connections, custom_config.timeout_seconds
    );

    // 演示默认服务
    let _service = default_service();
    println!("✓ SdForgeGrpcService (default) created");

    println!();
    println!("📖 gRPC Service Methods:");
    println!("  - Call(method)      -> CallResponse");
    println!("  - GetInfo()         -> InfoResponse");
    println!();
    println!("💡 启动服务器（绑定到 127.0.0.1:50051）:");
    println!("  // start_default(\"127.0.0.1:50051\").await?;");
    println!();
    println!("💡 测试客户端调用:");
    println!("  grpcurl -plaintext -d '{{\"method\":\"GET\"}}' localhost:50051 \\");
    println!("    sdforge.v1.SdForgeService/Call");
    println!();

    // 注意：实际启动会阻塞。以下代码在真实场景中取消注释即可运行。
    // start_default("127.0.0.1:50051").await?;

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sample_route() {
        let route = create_sample_route();
        // GrpcRoute fields are pub(crate), so we verify via Debug format
        let debug = format!("{:?}", route);
        assert!(
            debug.contains("SdForgeService"),
            "route should contain service name"
        );
    }

    #[test]
    fn test_default_server_config_values() {
        let config = default_server_config();
        assert_eq!(
            config.max_connections, 1000,
            "default max_connections should be 1000"
        );
        assert_eq!(config.timeout_seconds, 30, "default timeout should be 30s");
    }

    #[test]
    fn test_custom_server_config_values() {
        let config = custom_server_config();
        assert_eq!(config.max_connections, 200);
        assert_eq!(config.timeout_seconds, 60);
    }

    #[test]
    fn test_default_service_creation() {
        let _service = default_service();
        // SdForgeGrpcService derives Default; creation should succeed
    }

    #[tokio::test]
    async fn test_start_default_rejects_invalid_addr() {
        // Invalid address should return an error, not panic
        let result = start_default("not-a-valid-address").await;
        assert!(result.is_err(), "invalid address should produce an error");
    }
}

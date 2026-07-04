// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//!
//! # OpenAPI 自动生成基础示例
//!
//! 本模块展示 SDForge v0.2.0 引入的 OpenAPI 3.1 规范自动生成功能。
//!
//! ## 功能概述
//!
//! 启用 `openapi` feature 后，每个 `#[service_api]` 宏会在编译时通过
//! `inventory::submit!` 注册一条 `OpenApiRouteInfo`。运行时调用
//! `generate_openapi_spec()` 即可收集所有路由并生成完整规范。
//!
//! ## 启用方式
//!
//! ```toml
//! [dependencies]
//! sdforge = { version = "0.2", features = ["http", "openapi"] }
//! ```
//!
//! ## 基本用法
//!
//! ```rust,ignore
//! use sdforge::openapi::generate_openapi_spec;
//!
//! let spec = generate_openapi_spec();
//! let json = serde_json::to_string_pretty(&spec).unwrap();
//! println!("{json}");
//! ```
//!
//! ## 自定义元信息
//!
//! ```rust,ignore
//! use sdforge::openapi::OpenApiBuilder;
//!
//! let spec = OpenApiBuilder::new()
//!     .title("My Service")
//!     .version("2.0.0")
//!     .description("User-facing API")
//!     .build();
//! ```

use sdforge::openapi::{generate_openapi_spec, OpenApiBuilder, OpenApiRouteInfo};
use sdforge::prelude::*;

// ============================================================================
// 示例端点定义 — 这些端点会被宏自动注册到 OpenApiRouteInfo
// ============================================================================

/// 用户查询端点
///
/// 启用 `openapi` feature 后，此端点会自动出现在生成的 OpenAPI 规范中。
#[service_api(
    name = "openapi_get_user",
    version = "v1",
    path = "/openapi-demo/users/:id",
    method = "GET",
    description = "Get a user by ID for OpenAPI demo"
)]
async fn openapi_get_user(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": "OpenAPI Demo User",
        "source": "openapi_basic_example"
    }))
}

/// 用户列表端点
#[service_api(
    name = "openapi_list_users",
    version = "v1",
    path = "/openapi-demo/users",
    method = "GET",
    description = "List all users for OpenAPI demo"
)]
async fn openapi_list_users() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "users": [
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ],
        "total": 2
    }))
}

// ============================================================================
// OpenAPI 规范生成示例函数
// ============================================================================

/// 生成默认的 OpenAPI 规范
///
/// 使用默认标题 "SDForge API" 和 crate 版本号。
pub fn demo_default_spec() -> serde_json::Value {
    let spec = generate_openapi_spec();
    serde_json::to_value(&spec).expect("OpenApi spec should serialize to JSON")
}

/// 生成自定义的 OpenAPI 规范
///
/// 展示 `OpenApiBuilder` 链式调用定制 `info` 段。
pub fn demo_custom_spec() -> serde_json::Value {
    let spec = OpenApiBuilder::new()
        .title("Demo Service")
        .version("1.0.0")
        .description("OpenAPI generation demo from sdforge-examples")
        .build();
    serde_json::to_value(&spec).expect("OpenApi spec should serialize to JSON")
}

/// 将 OpenAPI 规范序列化为 JSON 字符串
///
/// 便于写入文件或返回给 HTTP 客户端。
pub fn demo_spec_to_json() -> String {
    let spec = generate_openapi_spec();
    serde_json::to_string_pretty(&spec).expect("OpenApi spec should serialize to JSON string")
}

// ============================================================================
// 手动注册路由（不使用 #[service_api] 宏的场景）
// ============================================================================

// 手动注册一条 OpenAPI 路由信息
//
// 对于不由 `#[service_api]` 宏声明的路由（如动态注册的中间件路由），
// 可以手动通过 `inventory::submit!` 注册 `OpenApiRouteInfo`。
inventory::submit!(
    OpenApiRouteInfo::new(
        "/openapi-demo/manual",
        "GET",
        "Manually registered route",
        "This route was registered via inventory::submit! directly, not via #[service_api]",
        "v1",
        &["manual", "demo"]
    )
);

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_spec_should_have_title() {
        let spec = demo_default_spec();
        let info = spec.get("info").expect("spec should have info section");
        let title = info.get("title").and_then(|t| t.as_str()).expect("info should have title");
        assert_eq!(title, "SDForge API");
    }

    #[test]
    fn custom_spec_should_reflect_builder_inputs() {
        let spec = demo_custom_spec();
        let info = spec.get("info").expect("spec should have info section");
        assert_eq!(
            info.get("title").and_then(|t| t.as_str()),
            Some("Demo Service")
        );
        assert_eq!(
            info.get("version").and_then(|v| v.as_str()),
            Some("1.0.0")
        );
        assert_eq!(
            info.get("description").and_then(|d| d.as_str()),
            Some("OpenAPI generation demo from sdforge-examples")
        );
    }

    #[test]
    fn spec_to_json_should_be_valid_json() {
        let json = demo_spec_to_json();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("output should be valid JSON");
        assert!(parsed.is_object(), "OpenAPI spec should be a JSON object");
    }

    #[test]
    fn manual_route_should_be_registered() {
        let spec = demo_default_spec();
        let paths = spec.get("paths").expect("spec should have paths section");
        assert!(
            paths.get("/openapi-demo/manual").is_some(),
            "manually registered route should appear in paths"
        );
    }
}

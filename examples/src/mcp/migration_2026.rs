// Copyright (c) 2026 Kirky.X
//!
//! # MCP 2026-07-28 迁移示例
//!
//! 本模块展示从 `mcp-sdk 0.0.3` 迁移到 `rmcp 0.16` 的完整流程，
//! 适配 MCP 2026-07-28 规范的无状态协议模型。
//!
//! ## BREAKING 变更概览
//!
//! | 旧版本 (v0.1.x)                      | 新版本 (v0.2.0)                              |
//! |--------------------------------------|----------------------------------------------|
//! | `mcp-sdk = "0.0.3"` 依赖             | `rmcp = "0.16"` 依赖                         |
//! | `initialize` 握手流程                | 移除，改用 `server/discover` 端点            |
//! | 有状态会话（StatefulServerHandler）  | 无状态适配层（`StatelessServerHandler`）    |
//! | `register_mcp(&mut Server)` 签名      | `register_mcp(&mut dyn McpToolRegistry)`      |
//!
//! ## 迁移步骤
//!
//! 1. 将 `Cargo.toml` 的 `mcp-sdk` 依赖替换为 `rmcp`（`features = ["server"]`）
//! 2. 将 `register_mcp(&mut Server)` 调用改为 `register_mcp(&mut dyn McpToolRegistry)`
//! 3. 移除 `initialize` 握手相关代码，改用 `server/discover` 端点
//! 4. 如需 MRTR 或缓存语义，引入对应模块

use sdforge::mcp::{McpHeaderInfo, StatelessServerHandler};
use sdforge::prelude::*;

// ============================================================================
// 无状态适配层示例
// ============================================================================

/// 演示 `StatelessServerHandler` 的构造
///
/// `StatelessServerHandler` 实现了 `rmcp::ServerHandler` trait，
/// 每个方法不依赖会话状态，适配 2026-07-28 规范的无状态协议模型。
///
/// 构造时需传入一个 `SdForgeMcpServer`（可通过 `sdforge::mcp::build()` 获取）。
pub fn demo_stateless_handler() -> StatelessServerHandler {
    StatelessServerHandler::new(sdforge::mcp::build())
}

// ============================================================================
// HTTP 头协议示例
// ============================================================================

/// 演示 `Mcp-Method` 和 `Mcp-Name` 头的预期格式
///
/// 无状态协议通过 HTTP 头传递方法与工具名。客户端请求需携带：
/// ```text
/// Mcp-Method: tools/call
/// Mcp-Name: get_user
/// ```
pub fn demo_expected_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Mcp-Method", "tools/call"),
        ("Mcp-Name", "get_user"),
    ]
}

/// 演示 `McpHeaderInfo` 的字段结构
///
/// `parse_mcp_headers` 解析请求头后返回此结构。缺失头时返回
/// `400 Bad Request`，与 2026-07-28 规范一致。
pub fn demo_header_info_shape() -> McpHeaderInfo {
    McpHeaderInfo {
        method: "tools/call".to_string(),
        tool_name: Some("get_user".to_string()),
    }
}

// ============================================================================
// 迁移后的工具定义示例
// ============================================================================

/// 迁移后的 MCP 工具端点
///
/// v0.2.0 中 `#[service_api]` 宏自动适配 rmcp，无需手动修改工具定义。
/// 工具通过 `tool_name` 属性自动注册到 MCP 服务器。
#[service_api(
    name = "migrated_get_user",
    version = "v1",
    path = "/migrated/users/:id",
    method = "GET",
    tool_name = "migrated_get_user",
    description = "Migrated MCP tool — no initialize handshake required"
)]
async fn migrated_get_user(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": "Migrated User",
        "protocol": "MCP 2026-07-28",
        "handshake": "none (stateless)"
    }))
}

/// 演示 `server/discover` 端点替代 `initialize`
///
/// 2026-07-28 规范移除了 `initialize` 握手，改用 `server/discover`
/// 端点暴露工具能力。SDForge 的 `StatelessServerHandler` 自动处理此端点。
#[service_api(
    name = "discover_endpoint",
    version = "v1",
    path = "/mcp/discover",
    method = "GET",
    tool_name = "discover",
    description = "server/discover endpoint — replaces initialize handshake"
)]
async fn discover_endpoint() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "protocol_version": "2026-07-28",
        "server_name": "sdforge-migrated",
        "capabilities": {
            "tools": {"list_changed": true},
            "resources": {"subscribe": false}
        }
    }))
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stateless_handler_should_construct() {
        let _handler = demo_stateless_handler();
    }

    #[test]
    fn expected_headers_should_contain_method_and_name() {
        let headers = demo_expected_headers();
        let method = headers
            .iter()
            .find(|(k, _)| *k == "Mcp-Method")
            .map(|(_, v)| *v);
        let name = headers
            .iter()
            .find(|(k, _)| *k == "Mcp-Name")
            .map(|(_, v)| *v);
        assert_eq!(method, Some("tools/call"));
        assert_eq!(name, Some("get_user"));
    }

    #[test]
    fn header_info_shape_should_match_protocol() {
        let info = demo_header_info_shape();
        assert_eq!(info.method, "tools/call");
        assert_eq!(info.tool_name.as_deref(), Some("get_user"));
    }
}

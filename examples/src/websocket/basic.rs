// Copyright (c) 2026 Kirky.X
//!
//! # 基础 WebSocket 示例
//!
//! 本模块展示基础的 WebSocket 连接和处理模式。
//!
//! ## WebSocket 连接流程
//!
//! 1. **客户端发起连接**
//!    ```javascript
//!    const ws = new WebSocket('ws://localhost:3000/ws/basic');
//!    ```
//!
//! 2. **服务器接受连接**
//!    - 验证请求
//!    - 升级协议
//!    - 建立连接
//!
//! 3. **双向消息传递**
//!    ```javascript
//!    ws.send('Hello Server!');
//!    ws.onmessage = (event) => console.log(event.data);
//!    ```
//!
//! 4. **关闭连接**
//!    ```javascript
//!    ws.close();
//!    ```
//!
//! ## 消息格式
//!
//! ### 文本消息
//! ```json
//! {
//!     "type": "message",
//!     "content": "Hello"
//! }
//! ```
//!
//! ### 心跳消息
//! ```json
//! {
//!     "type": "ping"
//! }
//! ```
//!
//! ## 错误处理
//!
//! WebSocket 连接可能遇到的错误：
//! - 连接超时
//! - 协议升级失败
//! - 服务器不可用

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 消息类型定义
// ============================================================================

/// WebSocket 消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    /// 消息类型
    #[serde(rename = "type")]
    pub msg_type: String,
    /// 消息内容
    pub content: String,
    /// 时间戳
    pub timestamp: Option<String>,
}

/// 状态更新消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    /// 状态类型
    pub status: String,
    /// 状态值
    pub value: serde_json::Value,
    /// 更新时间
    pub updated_at: String,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// 基础 WebSocket 端点
///
/// 建立基础的 WebSocket 连接。
///
/// # WebSocket URL
/// ```text
/// ws://localhost:3000/ws/basic
/// ```
///
/// # 客户端示例
/// ```javascript
/// const ws = new WebSocket('ws://localhost:3000/ws/basic');
///
/// ws.onopen = () => {
///     console.log('Connected to WebSocket');
///     ws.send(JSON.stringify({
///         type: 'message',
///         content: 'Hello!'
///     }));
/// };
///
/// ws.onmessage = (event) => {
///     const data = JSON.parse(event.data);
///     console.log('Received:', data);
/// };
/// ```
#[service_api(
    name = "websocket_basic",
    version = "v1",
    path = "/ws/basic",
    method = "GET",
    tool_name = "websocket_basic",
    description = "基础 WebSocket 端点"
)]
async fn websocket_basic() -> Result<String, ApiError> {
    Ok("WebSocket connection established".to_string())
}

/// 带认证的 WebSocket 端点
///
/// 需要在连接时提供认证信息。
///
/// # WebSocket URL
/// ```text
/// ws://localhost:3000/ws/auth?token=your_token
/// ```
///
/// # 认证消息格式
/// ```json
/// {
///     "type": "auth",
///     "token": "your_jwt_token"
/// }
/// ```
///
/// # 响应
/// ```json
/// {
///     "type": "auth_success",
///     "user_id": "user_123"
/// }
/// ```
#[service_api(
    name = "websocket_auth",
    version = "v1",
    path = "/ws/auth",
    method = "GET",
    tool_name = "websocket_auth",
    description = "带认证的 WebSocket 端点"
)]
async fn websocket_auth() -> Result<String, ApiError> {
    Ok("Authenticated WebSocket connection".to_string())
}

/// 实时状态订阅端点
///
/// 订阅实时状态更新。
///
/// # WebSocket URL
/// ```text
/// ws://localhost:3000/ws/subscribe
/// ```
///
/// # 订阅消息
/// ```json
/// {
///     "type": "subscribe",
///     "channel": "status_updates"
/// }
/// ```
///
/// # 推送消息格式
/// ```json
/// {
///     "type": "status_update",
///     "status": "cpu_usage",
///     "value": 45.2,
///     "updated_at": "2024-01-17T12:00:00Z"
/// }
/// ```
#[service_api(
    name = "websocket_subscribe",
    version = "v1",
    path = "/ws/subscribe",
    method = "GET",
    tool_name = "websocket_subscribe",
    description = "状态订阅 WebSocket 端点"
)]
async fn websocket_subscribe() -> Result<String, ApiError> {
    Ok("WebSocket subscription established".to_string())
}

/// 获取 WebSocket 连接信息
///
/// 返回当前连接的相关信息。
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/ws/info
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "websocket_version": "13",
///     "supported_protocols": ["json", "binary"],
///     "max_message_size": 65536
/// }
/// ```
#[service_api(
    name = "websocket_info",
    version = "v1",
    path = "/ws/info",
    method = "GET",
    tool_name = "websocket_info",
    description = "获取 WebSocket 连接信息"
)]
async fn websocket_info() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "websocket_version": "13",
        "supported_protocols": ["json", "binary"],
        "max_message_size": 65536,
        "ping_interval_seconds": 30,
        "connection_timeout_seconds": 60
    }))
}

// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 聊天 WebSocket 示例
//!
//! 本模块展示如何使用 WebSocket 实现实时聊天功能。
//!
//! ## 聊天功能概述
//!
//! ### 消息类型
//!
//! | 类型 | 说明 | 方向 |
//! |------|------|------|
//! | `message` | 聊天消息 | 双向 |
//! | `join` | 加入房间 | 客户端 -> 服务器 |
//! | `leave` | 离开房间 | 客户端 -> 服务器 |
//! | `typing` | 正在输入 | 双向 |
//! | `online` | 用户上线 | 服务器 -> 客户端 |
//! | `offline` | 用户下线 | 服务器 -> 客户端 |
//!
//! ## 消息格式
//!
//! ### 发送消息
//! ```json
//! {
//!     "type": "message",
//!     "room": "general",
//!     "content": "Hello everyone!",
//!     "sender": "user_123"
//! }
//! ```
//!
//! ### 接收消息
//! ```json
//! {
//!     "type": "message",
//!     "room": "general",
//!     "content": "Hello everyone!",
//!     "sender": "user_123",
//!     "timestamp": "2024-01-17T12:00:00Z"
//! }
//! ```
//!
//! ## 使用流程
//!
//! 1. 连接到 WebSocket
//! 2. 发送 join 消息加入房间
//! 3. 发送和接收消息
//! 4. 发送 leave 消息离开房间
//! 5. 关闭连接

use sdforge::prelude::*;
use sdforge::serde::{Deserialize, Serialize};

// ============================================================================
// 聊天消息类型定义
// ============================================================================

/// 聊天消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 目标房间
    pub room: String,
    /// 消息内容
    pub message: String,
    /// 发送者 ID (可选)
    pub sender: Option<String>,
}

/// 加入房间请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRoomRequest {
    /// 房间名称
    pub room: String,
    /// 用户 ID
    pub user_id: String,
    /// 用户昵称
    pub nickname: String,
}

/// 离开房间请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRoomRequest {
    /// 房间名称
    pub room: String,
    /// 用户 ID
    pub user_id: String,
}

/// 消息响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    /// 消息 ID
    pub id: String,
    /// 房间名称
    pub room: String,
    /// 消息内容
    pub message: String,
    /// 发送者
    pub sender: String,
    /// 时间戳
    pub timestamp: String,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// 聊天 WebSocket 端点
///
/// 实时聊天服务的主 WebSocket 端点。
///
/// # WebSocket URL
/// ```text
/// ws://localhost:3000/ws/chat
/// ```
///
/// # 完整聊天流程
///
/// ### 1. 连接
/// ```javascript
/// const ws = new WebSocket('ws://localhost:3000/ws/chat');
/// ```
///
/// ### 2. 加入房间
/// ```javascript
/// ws.send(JSON.stringify({
///     type: 'join',
///     room: 'general',
///     user_id: 'user_123',
///     nickname: 'John'
/// }));
/// ```
///
/// ### 3. 发送消息
/// ```javascript
/// ws.send(JSON.stringify({
///     type: 'message',
///     room: 'general',
///     content: 'Hello everyone!',
///     sender: 'user_123'
/// }));
/// ```
///
/// ### 4. 接收消息
/// ```javascript
/// ws.onmessage = (event) => {
///     const msg = JSON.parse(event.data);
///     if (msg.type === 'message') {
///         console.log(`${msg.sender}: ${msg.content}`);
///     }
/// };
/// ```
///
/// ### 5. 离开房间
/// ```javascript
/// ws.send(JSON.stringify({
///     type: 'leave',
///     room: 'general',
///     user_id: 'user_123'
/// }));
/// ```
#[forge(
    name = "chat_ws",
    version = "v1",
    path = "/ws/chat",
    method = "GET",
    tool_name = "chat_ws",
    description = "聊天 WebSocket 端点"
)]
async fn chat_ws() -> Result<String, ApiError> {
    Ok("WebSocket chat connection".to_string())
}

/// 发送聊天消息 (HTTP API)
///
/// 通过 HTTP API 发送聊天消息（备用方式）。
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/chat/message \
///   -H "Content-Type: application/json" \
///   -d '{
///     "room": "general",
///     "message": "Hello via HTTP!",
///     "sender": "user_123"
///   }'
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "id": "msg_abc123",
///     "room": "general",
///     "message": "Hello via HTTP!",
///     "sender": "user_123",
///     "timestamp": "2024-01-17T12:00:00Z"
/// }
/// ```
#[forge(
    name = "send_message",
    version = "v1",
    path = "/chat/message",
    method = "POST",
    tool_name = "send_message",
    description = "发送聊天消息 (HTTP)"
)]
async fn send_message(message: ChatMessage) -> Result<MessageResponse, ApiError> {
    let response = MessageResponse {
        id: format!("msg_{}", uuid::Uuid::new_v4()),
        room: message.room,
        message: message.message,
        sender: message.sender.unwrap_or_else(|| "anonymous".to_string()),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    Ok(response)
}

/// 获取房间信息
///
/// 查看聊天室的基本信息。
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/chat/rooms/general
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "room": "general",
///     "description": "General chat room",
///     "member_count": 25,
///     "created_at": "2024-01-01T00:00:00Z"
/// }
/// ```
#[forge(
    name = "get_room_info",
    version = "v1",
    path = "/chat/rooms/:room",
    method = "GET",
    tool_name = "get_room_info",
    description = "获取房间信息"
)]
async fn get_room_info(room: String) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "room": room,
        "description": format!("Chat room: {}", room),
        "member_count": 0,
        "message_count": 0,
        "created_at": "2024-01-01T00:00:00Z"
    }))
}

/// 获取房间列表
///
/// 列出所有可用的聊天室。
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/chat/rooms
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "rooms": [
///         {"name": "general", "members": 25},
///         {"name": "tech", "members": 10},
///         {"name": "random", "members": 15}
///     ]
/// }
/// ```
#[forge(
    name = "list_rooms",
    version = "v1",
    path = "/chat/rooms",
    method = "GET",
    tool_name = "list_rooms",
    description = "获取房间列表"
)]
async fn list_rooms() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "rooms": [
            {"name": "general", "description": "General chat", "members": 25},
            {"name": "tech", "description": "Tech discussions", "members": 10},
            {"name": "random", "description": "Off-topic chat", "members": 15}
        ]
    }))
}

/// 获取聊天历史
///
/// 获取房间的历史消息。
///
/// # HTTP 用法
/// ```bash
/// curl "http://localhost:3000/api/v1/chat/rooms/general/history?limit=50&before=2024-01-17T12:00:00Z"
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "room": "general",
///     "messages": [
///         {
///             "id": "msg_001",
///             "sender": "user_123",
///             "content": "Hello!",
///             "timestamp": "2024-01-17T11:55:00Z"
///         }
///     ],
///     "has_more": true
/// }
/// ```
#[forge(
    name = "get_chat_history",
    version = "v1",
    path = "/chat/rooms/:room/history",
    method = "GET",
    tool_name = "get_chat_history",
    description = "获取聊天历史"
)]
async fn get_chat_history(
    room: String,
    limit: Option<u32>,
    before: Option<String>,
) -> Result<serde_json::Value, ApiError> {
    let limit = limit.unwrap_or(50).min(100);

    Ok(serde_json::json!({
        "room": room,
        "messages": [],
        "limit": limit,
        "before": before,
        "has_more": false
    }))
}

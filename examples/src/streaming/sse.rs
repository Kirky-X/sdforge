// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # Server-Sent Events (SSE) 示例
//!
//! 本模块展示如何实现 SSE 流式传输。
//!
//! ## SSE 简介
//!
//! Server-Sent Events (SSE) 是一种服务器推送技术，允许服务器通过 HTTP 连接
//! 向客户端发送实时更新。
//!
//! ## 特点
//!
//! - **单向通信** - 服务器只能发送，客户端只能接收
//! - **自动重连** - 连接断开时浏览器自动重连
//! - **简单协议** - 基于纯文本，易于调试
//!
//! ## SSE vs WebSocket
//!
//! | 特性 | SSE | WebSocket |
//! |------|-----|----------|
//! | 方向 | 单向 | 双向 |
//! | 协议 | HTTP | ws:// |
//! | 重连 | 自动 | 手动 |
//! | 兼容性 | 现代浏览器 | 所有浏览器 |
//!
//! ## 事件格式
//!
//! ### 标准事件
//! ```text
//! event: message
//! data: {"content": "Hello"}
//!
//! ```
//!
//! ### 命名事件
//! ```text
//! event: update
//! data: {"type": "progress", "value": 50}
//!
//! ```
//!
//! ### 多行数据
//! ```text
//! data: {"first": "line"
//! data: ,"second": "line"
//! data: }
//!
//! ```
//!
//! ## 客户端示例
//!
//! ```javascript
//! const eventSource = new EventSource('/api/v1/stream/events');
//!
//! eventSource.onmessage = (event) => {
//!     console.log('Message:', event.data);
//! };
//!
//! eventSource.addEventListener('update', (event) => {
//!     console.log('Update:', event.data);
//! });
//!
//! eventSource.onerror = (error) => {
//!     console.error('SSE Error:', error);
//! };
//! ```

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 流事件类型定义
// ============================================================================

/// 流式事件结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// 事件 ID
    pub id: String,
    /// 事件类型
    #[serde(rename = "type")]
    pub event_type: String,
    /// 事件数据
    pub data: serde_json::Value,
    /// 时间戳
    pub timestamp: String,
}

/// 进度更新结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    /// 任务 ID
    pub task_id: String,
    /// 当前进度 (0-100)
    pub progress: u32,
    /// 状态消息
    pub message: String,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// SSE 流式事件端点
///
/// 建立 SSE 连接，接收实时事件流。
///
/// # SSE URL
/// ```text
/// http://localhost:3000/api/v1/stream/events
/// ```
///
/// # 事件格式
/// ```text
/// event: message
/// data: {"id":"1","type":"message","content":"Event 1","timestamp":"..."}
///
/// event: message
/// data: {"id":"2","type":"message","content":"Event 2","timestamp":"..."}
///
/// ```
///
/// # 客户端示例
/// ```javascript
/// const eventSource = new EventSource('http://localhost:3000/api/v1/stream/events');
///
/// eventSource.onmessage = (event) => {
///     const data = JSON.parse(event.data);
///     console.log(`Event ${data.id}: ${data.content}`);
/// };
/// ```
#[forge(
    name = "sse_stream",
    version = "v1",
    path = "/stream/events",
    method = "GET",
    tool_name = "sse_stream",
    description = "SSE 流式事件端点",
    streaming = true
)]
async fn sse_stream() -> Result<String, ApiError> {
    Ok("SSE stream established".to_string())
}

/// 事件流端点
///
/// 发送一系列事件。
///
/// # SSE 事件格式
/// ```text
/// event: event
/// data: {"event_id":"evt_001","type":"notification","content":"New notification"}
///
/// event: event
/// data: {"event_id":"evt_002","type":"alert","content":"Alert message"}
///
/// ```
#[forge(
    name = "event_stream",
    version = "v1",
    path = "/stream/subscribe",
    method = "GET",
    tool_name = "event_stream",
    description = "事件流订阅端点",
    streaming = true
)]
async fn event_stream() -> Result<String, ApiError> {
    Ok("Event stream started".to_string())
}

/// 进度流端点
///
/// 实时推送任务进度更新。
///
/// # SSE URL
/// ```text
/// http://localhost:3000/api/v1/stream/progress/:task_id
/// ```
///
/// # SSE 事件格式
/// ```text
/// event: progress
/// data: {"task_id":"task_123","progress":0,"message":"Starting..."}
///
/// event: progress
/// data: {"task_id":"task_123","progress":25,"message":"Processing step 1"}
///
/// event: progress
/// data: {"task_id":"task_123","progress":50,"message":"Processing step 2"}
///
/// event: progress
/// data: {"task_id":"task_123","progress":75,"message":"Processing step 3"}
///
/// event: progress
/// data: {"task_id":"task_123","progress":100,"message":"Complete!"}
///
/// ```
///
/// # 客户端示例
/// ```javascript
/// const taskId = 'task_123';
/// const eventSource = new EventSource(`http://localhost:3000/api/v1/stream/progress/${taskId}`);
///
/// eventSource.addEventListener('progress', (event) => {
///     const update = JSON.parse(event.data);
///     console.log(`Progress: ${update.progress}% - ${update.message}`);
/// });
///
/// eventSource.addEventListener('complete', (event) => {
///     console.log('Task completed!');
///     eventSource.close();
/// });
/// ```
#[forge(
    name = "progress_stream",
    version = "v1",
    path = "/stream/progress/:task_id",
    method = "GET",
    tool_name = "progress_stream",
    description = "进度流端点",
    streaming = true
)]
async fn progress_stream(task_id: String) -> Result<String, ApiError> {
    Ok(format!("Progress stream for task: {}", task_id))
}

/// 实时数据流端点
///
/// 推送实时数据更新。
///
/// # SSE 事件格式
/// ```text
/// event: data_update
/// data: {"source":"sensor_1","value":25.5,"unit":"celsius"}
///
/// event: data_update
/// data: {"source":"sensor_2","value":72.1,"unit":"percent"}
///
/// ```
#[forge(
    name = "data_stream",
    version = "v1",
    path = "/stream/data",
    method = "GET",
    tool_name = "data_stream",
    description = "实时数据流端点",
    streaming = true
)]
async fn data_stream() -> Result<String, ApiError> {
    Ok("Data stream started".to_string())
}

/// 心跳端点
///
/// 发送心跳信号，用于保持连接和健康检查。
///
/// # SSE 事件格式
/// ```text
/// event: heartbeat
/// data: {"timestamp":"2024-01-17T12:00:00Z","latency_ms":15}
///
/// ```
#[forge(
    name = "heartbeat_stream",
    version = "v1",
    path = "/stream/heartbeat",
    method = "GET",
    tool_name = "heartbeat_stream",
    description = "心跳流端点",
    streaming = true
)]
async fn heartbeat_stream() -> Result<String, ApiError> {
    Ok("Heartbeat stream started".to_string())
}

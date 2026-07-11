// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 流式传输示例模块
//!
//! 本模块展示 SDForge 框架的流式传输功能。
//!
//! ## 流式传输类型
//!
//! ### 1. Server-Sent Events (SSE)
//!
//! 服务器向客户端推送事件，单向通信。
//!
//! ### 2. 分块传输编码
//!
//! HTTP 分块响应，用于大文件或长响应。
//!
//! ## SSE 格式
//!
//! ```text
//! event: message
//! data: {"content": "Hello"}
//!
//! event: message
//! data: {"content": "World"}
//!
//! ```
//!
//! ## 适用场景
//!
//! - 实时通知
//! - 进度更新
//! - 实时数据监控
//! - AI 响应流式输出

pub mod sse;

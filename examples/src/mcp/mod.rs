// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # MCP 协议示例模块
//!
//! 本模块展示 SDForge 框架的 MCP (Model Context Protocol) 功能。
//!
//! ## MCP 简介
//!
//! MCP 是一种用于 AI 模型与工具之间通信的协议。通过 SDForge，你可以：
//!
//! - 定义可被 AI 模型调用的工具
//! - 自动生成工具的元数据
//! - 通过统一的 API 同时支持 HTTP 和 MCP
//!
//! ## 核心概念
//!
//! ### 工具定义
//!
//! 使用 `#[forge]` 宏定义 MCP 工具：
//!
//! ```text
//! # use serde_json;
//! # use sdforge::prelude::ApiError;
//! #[forge(
//!     name = "tool_name",
//!     tool_name = "mcp_tool_name",  // MCP 工具名
//!     description = "工具描述",
//!     ...
//! )]
//! async fn my_tool(param: String) -> Result<serde_json::Value, ApiError> {
//!     // 实现
//! }
//! ```
//!
//! ### 工具调用
//!
//! MCP 工具通过 JSON 调用：
//!
//! ```json
//! {
//!     "tool": "tool_name",
//!     "input": {
//!         "param": "value"
//!     }
//! }
//! ```

pub mod migration_2026;
pub mod mrtr_example;
pub mod tool_definition;
pub mod tool_registration;
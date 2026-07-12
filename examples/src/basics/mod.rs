// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 基础示例模块 (Basics)
//!
//! 本模块展示 SDForge 框架的核心基础功能，包括：
//!
//! - **简单 API 定义** - 使用 `#[forge]` 宏定义基本服务 API
//! - **类型系统** - SDForge 提供的核心类型及其用法
//! - **响应构建** - 如何构建各种类型的响应
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use sdforge::prelude::*;
//!
//! #[forge(
//!     name = "hello_world",
//!     version = "v1",
//!     path = "/hello",
//!     method = "GET",
//!     tool_name = "hello_world",
//!     description = "简单的 Hello World API"
//! )]
//! async fn hello_world() -> Result<String, ApiError> {
//!     Ok("Hello, World!".to_string())
//! }
//! ```
//!
//! ## 模块内容
//!
//! - \[`simple_api`\] - 简单 API 定义示例
//! - \[`types_and_errors`\] - 类型和错误处理
//! - \[`response_building`\] - 响应构建示例

pub mod response_building;
pub mod simple_api;
pub mod types_and_errors;

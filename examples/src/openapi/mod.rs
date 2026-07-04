// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//!
//! # OpenAPI 自动生成示例模块
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
//! ## 包含的示例
//!
//! - [`basic`](basic) - OpenAPI 基础用法：默认规范生成、自定义 builder、手动注册路由

pub mod basic;

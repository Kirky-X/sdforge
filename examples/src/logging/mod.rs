// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! # 结构化日志示例模块
//!
//! 本模块展示 inklog `LoggerManager` 的使用方式（自研 `StructuredLogger`
//! 栈已在 0.5.0-rc 期删除）。
//!
//! ## 涵盖接口
//!
//! - [`init_inklog_logger`](sdforge::inklog::init_inklog_logger) — 安装
//!   inklog 为全局 `log` 后端（facade 调用零改动）
//! - [`LoggerManager::from_file`] — 从含轮转配置的 TOML 构造
//! - [`LoggerManager::builder`] — 编程式构造
//! - [`LoggerManager::set_level`] — 运行时调级

/// 结构化日志示例
pub mod structured;

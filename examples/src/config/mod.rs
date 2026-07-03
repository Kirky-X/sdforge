// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//!
//! # 配置管理示例模块
//!
//! 本模块展示 SDForge 配置管理的使用方式。
//!
//! ## 涵盖接口
//!
//! - [`AppConfig`] — 应用主配置
//! - [`AppConfigBuilder`] — Builder 模式构建配置
//! - [`ServerConfig`], [`AuthConfig`], [`TimeoutConfig`] — 子配置
//! - [`Config`] trait — confers 自动加载 trait

/// 应用配置构建与加载示例
pub mod app_config;

/// confers `#[derive(Config)]` 宏完整示例
pub mod derive_config;

/// confers `ConfigBuilder<T>` 流式 API 完整示例
pub mod config_builder;

/// 配置热重载示例
pub mod hot_reload;

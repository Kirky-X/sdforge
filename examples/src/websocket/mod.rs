// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # WebSocket 示例模块
//!
//! 本模块展示 SDForge 框架的 WebSocket 支持。
//!
//! ## WebSocket 简介
//!
//! WebSocket 是一种双向通信协议，允许服务器主动向客户端推送数据。
//!
//! ## 特点
//!
//! - **全双工通信** - 客户端和服务器可以同时发送消息
//! - **持久连接** - 连接建立后保持打开状态
//! - **低延迟** - 适合实时应用
//!
//! ## 适用场景
//!
//! - 实时聊天应用
//! - 实时数据监控
//! - 在线游戏
//! - 协作编辑

pub mod basic;
pub mod chat;

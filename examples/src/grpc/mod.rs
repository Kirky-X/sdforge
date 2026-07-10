// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//!
//! # gRPC 示例模块
//!
//! 本模块展示 SDForge gRPC 协议的使用方式。
//!
//! ## 涵盖接口
//!
//! - \[`GrpcRoute`\] — gRPC 路由注册
//! - \[`GrpcServerConfig`\] — 服务器配置（连接数、超时、JWT 认证）
//! - \[`build_server`\] — 构建并启动 gRPC 服务
//! - \[`SdForgeGrpcService`\] — 默认服务实现

/// gRPC 服务端构建示例
pub mod server;

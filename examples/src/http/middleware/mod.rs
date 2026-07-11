// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 中间件示例模块
//!
//! 本模块展示 HTTP 中间件的使用方式。
//!
//! ## 中间件类型
//!
//! ### 1. 请求日志中间件
//!
//! 记录每个请求的详细信息，用于调试和监控。
//!
//! ### 2. CORS 中间件
//!
//! 处理跨域资源共享 (Cross-Origin Resource Sharing)。
//!
//! ### 3. 认证中间件
//!
//! 验证请求的认证信息。
//!
//! ### 4. 速率限制中间件
//!
//! 控制请求频率。
//!
//! ## 使用方式
//!
//! 中间件通常在应用初始化时配置：
//!
//! ```rust,ignore
//! // 配置 CORS
//! let cors = CorsLayer::new()
//!     .allow_origin("https://example.com")
//!     .allow_methods([Method::GET, Method::POST]);
//!
//! // 添加到路由
//! app.layer(cors);
//! ```

pub mod cors;

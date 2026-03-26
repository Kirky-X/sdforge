// Copyright (c) 2026 Kirky.X
//!
//! # 安全模块示例
//!
//! 本模块展示 SDForge 框架的安全功能，包括：
//!
//! ## 安全功能
//!
//! - **API Key 认证** - 使用 API 密钥进行身份验证
//! - **速率限制** - 控制 API 请求频率
//! - **授权管理** - 权限和角色控制
//! - **审计日志** - 记录操作日志
//!
//! ## 认证方式
//!
//! ### 1. API Key 认证
//!
//! 通过 HTTP 头传递 API 密钥：
//!
//! ```bash
//! curl -H "X-API-Key: your_api_key" http://localhost:3000/api/v1/protected/resource
//! ```
//!
//! ### 2. Bearer Token 认证
//!
//! 使用 JWT 或其他令牌：
//!
//! ```bash
//! curl -H "Authorization: Bearer your_token" http://localhost:3000/api/v1/protected/resource
//! ```
//!
//! ## 速率限制
//!
//! | 级别 | 请求数/分钟 | 适用场景 |
//! |------|------------|---------|
//! | 标准 | 60 | 普通用户 |
//! | 严格 | 10 | 敏感操作 |
//! | 宽松 | 600 | API 用户 |

pub mod api_key;
pub mod rate_limiting;

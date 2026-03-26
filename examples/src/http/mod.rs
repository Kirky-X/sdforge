// Copyright (c) 2026 Kirky.X
//!
//! # HTTP 协议示例模块
//!
//! 本模块展示 SDForge 框架的 HTTP 服务器功能，包括：
//!
//! ## 模块结构
//!
//! - [`routing`](routing) - HTTP 路由示例
//!   - [`path_params`](routing/path_params) - 路径参数提取
//!   - [`query_params`](routing/query_params) - 查询参数提取
//! - [`middleware`](middleware) - 中间件示例
//!   - [`cors`](middleware/cors) - CORS 跨域资源共享
//!
//! ## HTTP 方法
//!
//! SDForge 支持标准的 HTTP 方法：
//! - `GET` - 获取资源
//! - `POST` - 创建资源
//! - `PUT` - 更新资源
//! - `DELETE` - 删除资源
//!
//! ## 路由匹配规则
//!
//! 1. 精确匹配优先于参数匹配
//! 2. 路径参数按名称自动提取
//! 3. 查询参数自动解析为函数参数

pub mod routing;
pub mod middleware;

// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # HTTP 路由示例模块
//!
//! 本模块展示 HTTP 路由的各种配置和使用方式。
//!
//! ## 路由基础
//!
//! ### 路径参数
//!
//! 路径参数使用 `:param_name` 语法定义，参数名必须与函数参数名匹配。
//!
//! ```bash
//! GET /users/:user_id/posts/:post_id
//! ```
//!
//! 函数签名:
//! ```text
//! async fn get_user_post(user_id: u64, post_id: u64) -> Result<...>
//! ```
//!
//! ### 嵌套资源
//!
//! RESTful API 推荐使用嵌套路由表示资源关系：
//!
//! - `/users/:user_id` - 用户的资源
//! - `/users/:user_id/posts` - 用户的帖子
//! - `/users/:user_id/posts/:post_id` - 用户的指定帖子

pub mod path_params;
pub mod query_params;
// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 多协议客户端 SDK 生成（`sdk` feature，蕴含 `openapi` + `cli`）。
//!
//! 从 inventory 注册表（openapi `OpenApiRouteInfo` 的 HTTP 面 + `grpc`
//! feature 下的 gRPC 方法清单）生成客户端产物：
//!
//! - **Rust**：零外部依赖单文件 client——`Transport` trait（RPITIT 返回
//!   `impl Future`，`base_url` 由客户端 `call` 拼进完整 URL）+ 每路由一个
//!   方法；`--reqwest` 时追加 `#[cfg(feature = "reqwest")]` 的
//!   `ReqwestTransport` 实现（生成物引用 `reqwest` crate，由使用方的 Cargo
//!   feature 门控）。返回体为原始 JSON 字符串，反序列化为具体类型由使用方
//!   按自身模型做（生成器不掌握返回类型的 Rust 路径）。
//! - **TypeScript**：`fetch` 单文件 client + 每路由参数/载荷 interface
//!   （类型定义内嵌，无需独立 dts 构建链）。
//!
//! 渲染是确定性的：inventory 迭代序不可依赖（链接期顺序），产物渲染前按
//! `(method, path)` 稳定排序，同一路由集合恒产出同一文本（快照测试锁定）。
//!
//! CLI 经保留子命令 `sdk` 输出（`sdk --lang rust|typescript|all
//! --output-dir <dir>`），与 `docs` 同一拦截范式。产物快照测试锁定渲染
//! 稳定性；Rust 产物在测试中以 `rustc --emit=metadata` 做真实编译冒烟。

mod generator;
mod subcommand;

pub use generator::{
    ClientRoute, GrpcMethodInfo, collect_grpc_methods, collect_routes, generate_rust_client,
    generate_typescript_client,
};
pub use subcommand::{SDK_LANG_VALUES, sdk_subcommand, sdk_subcommand_definition};

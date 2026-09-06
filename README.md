<div align="center">

<img src="docs/assets/sdforge.png" alt="SDForge Logo" width="200">

[![CI Status](https://github.com/Kirky-X/sdforge/actions/workflows/ci.yml/badge.svg)](https://github.com/Kirky-X/sdforge/actions/workflows/ci.yml) [![Version](https://img.shields.io/crates/v/sdforge.svg)](https://crates.io/crates/sdforge) [![Docs.rs](https://docs.rs/sdforge/badge.svg)](https://docs.rs/sdforge) [![Downloads](https://img.shields.io/crates/d/sdforge.svg)](https://crates.io/crates/sdforge) [![License](https://img.shields.io/crates/l/sdforge.svg)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.97.1%2B-orange.svg)](https://www.rust-lang.org/) [![Coverage](https://codecov.io/gh/Kirky-X/sdforge/branch/main/graph/badge.svg)](https://codecov.io/gh/Kirky-X/sdforge)

**中文** | [English](README_EN.md)

**SDForge** 是一个基于 Rust 的声明式 SDK 框架，利用过程宏从统一的函数注解自动生成多协议服务接口（HTTP + MCP + gRPC + WebSocket + CLI）。其核心创新在于通过 Cargo features 进行编译时协议选择——未使用的协议将产生零编译代码。

[✨ 功能特性](#-功能特性) • [🚀 快速开始](#-快速开始) • [📚 文档](#-文档) • [💻 示例](#-示例) • [🤝 参与贡献](#-参与贡献)

</div>

---

## 📋 目录

<details open>

- [✨ 功能特性](#-功能特性)
- [🚀 快速开始](#-快速开始)
  - [📦 安装](#-安装)
  - [💡 基本用法](#-基本用法)
  - [📁 模块前缀](#-模块前缀)
  - [🔢 多版本管理](#-多版本管理)
  - [🛤️ 路径参数](#️-路径参数)
  - [⚠️ 错误处理](#️-错误处理)
  - [🔧 `#[forge]` 宏参数](#-forge-宏参数)
  - [🌐 协议组合](#-协议组合)
  - [🛰️ gRPC Dispatch](#️-grpc-dispatch)
  - [🖥️ CLI Dispatch](#️-cli-dispatch)
- [🎨 特性标志](#-特性标志)
- [📚 文档](#-文档)
- [💻 示例](#-示例)
- [🏗️ 架构](#️-架构)
- [📜 OpenAPI 自动生成](#-openapi-自动生成)
- [🔄 MCP 2026-07-28 迁移指南](#-mcp-2026-07-28-迁移指南)
- [🚀 生产部署](#-生产部署)
- [🐛 故障排查](#-故障排查)
- [🧪 测试](#-测试)
- [📊 性能](#-性能)
- [🔒 安全](#-安全)
- [🗺️ 开发路线图](#️-开发路线图)
- [🤝 参与贡献](#-参与贡献)
- [📋 更新日志](#-更新日志)
- [📄 许可证](#-许可证)
- [🙏 致谢](#-致谢)
- [📞 联系与支持](#-联系与支持)
- [⭐ Star 历史](#-star-历史)

</details>

---

## ✨ 功能特性

| 特性 | 说明 |
|------|------|
| **🎯 统一接口定义** | 针对 HTTP、MCP、gRPC、WebSocket、CLI 的单一宏配置 |
| **⚡ 编译时协议选择** | 通过 Feature 控制代码生成，未使用的协议零运行时开销 |
| **🔒 类型安全** | 接口定义的编译时验证 |
| **🌐 多协议支持** | HTTP (Axum)、MCP (rmcp 3.2)、gRPC (tonic)、WebSocket、SSE 流式传输、CLI (clap) |
| **🧩 模块化设计** | 基于 Feature 的架构，允许仅选择所需功能 |
| **🛡️ 安全特性** | 内置认证（Bearer/API Key）、限流（limiteron）、审计日志 |
| **💾 缓存** | 基于内存缓存（oxcache），无需外部数据库 |
| **🔧 配置管理** | 自包含的 TOML 配置（无需外部配置中心） |
| **📊 版本控制** | 内置 API 版本管理 |
| **📜 OpenAPI 自动生成** | 基于 utoipa 5.5 生成 OpenAPI 3.1 规范 |
| **🌐 国际化** | 基于 ICU4X 2.x 的本地化支持（`i18n` feature） |

### 🔌 可选功能

以下能力均通过 Cargo feature 按需启用，详见[特性标志](#-特性标志)：

| 可选功能 | 对应 Feature | 说明 |
|----------|--------------|------|
| HTTP 服务器 | `http` | Axum 0.8 路由、中间件、版本路由 |
| MCP 协议 | `mcp` | rmcp 官方 SDK，2026-07-28 规范（无状态 HTTP 头协议、MRTR、缓存语义） |
| SSE 流式传输 | `streaming` | SSE 事件流与流式响应构建 |
| WebSocket | `websocket` | 连接管理、广播、消息解析 |
| gRPC | `grpc` | tonic 服务、统一 handler dispatch |
| CLI | `cli` | clap 命令行集成、一站式 `CliBuilder::execute()` |
| OpenAPI 3.1 | `openapi` | 编译时注册路由信息，运行时生成规范 |
| 统一文档输出 | `docs` | Swagger UI + CLI/MCP Markdown |
| 认证与审计 | `security` | API Key / JWT Bearer、审计日志、安全头 |
| 限流 | `ratelimit` / `ratelimit-http` | limiteron 统一限流（核心 / Tower 中间件） |
| 缓存 | `cache` | oxcache 内存缓存（LRU、模式失效、统计） |
| 响应时间戳 | `timestamp` | 自动向响应添加时间戳 |
| 结构化日志 | `logging` | 结构化请求日志 |
| inklog 集成 | `inklog` | 桥接到 inklog LoggerManager 结构化日志管道 |
| 国际化 | `i18n` | ICU4X 本地化格式化与 Accept-Language 解析 |
| SIMD JSON | `simd-json` | SIMD 加速 JSON 序列化 |

#### 🆕 Phase 1 架构改进

近期的架构增强包括：

- **🔄 统一注册系统** — 通过 trait 抽象与过程宏，消除 HTTP、MCP、WebSocket、gRPC 模块间 95+ 行重复代码
- **⚙️ 模块化配置管理** — 配置重构为独立模块（app、cache、security），集中默认值并支持 Builder 模式
- **🔐 增强安全模块** — API Key 版本管理、LRU 缓存、带审计日志的密钥轮换、完善的安全头配置
- **💾 高级缓存** — 基于模式的缓存失效、键规范化、批量操作与统计信息跟踪

---

## 🚀 快速开始

### 📦 安装

```bash
cargo add sdforge
```

或手动添加到 `Cargo.toml`：

```toml
[dependencies]
sdforge = { version = "0.5.0-rc.2", features = ["http"] }
```

> 注意：`sdforge` 默认不启用任何特性（`default = []`），需按需显式启用协议特性。

### 💡 基本用法

使用单个宏定义你的 API：

```rust
use sdforge::prelude::*;

#[forge(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user",
    description = "Get a user by ID"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    Ok(User { id, name: "Test".into() })
}

#[tokio::main]
async fn main() {
    let app = sdforge::http::build();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### 📁 模块前缀

使用模块前缀对相关 API 进行分组：

```rust
#[service_module(prefix = "/auth")]
mod auth_api {
    use super::*;

    #[forge(
        name = "login",
        version = "v1",
        path = "/login",
        method = "POST"
    )]
    async fn login(credentials: Credentials) -> Result<Token, ApiError> {
        Ok(Token::new())
    }

    #[forge(
        name = "logout",
        version = "v1",
        path = "/logout",
        method = "POST"
    )]
    async fn logout() -> Result<(), ApiError> {
        Ok(())
    }
}
```

这将生成端点：

- `/auth/api/v1/login`
- `/auth/api/v1/logout`

### 🔢 多版本管理

同时支持多个 API 版本：

```rust
#[forge(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user_v1"
)]
async fn get_user_v1(id: u64) -> Result<UserV1, ApiError> {
    Ok(UserV1 { id, name: "John Doe".into() })
}

#[forge(
    name = "get_user",
    version = "v2",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user_v2"
)]
async fn get_user_v2(id: u64) -> Result<UserV2, ApiError> {
    Ok(UserV2 { id, first_name: "John".into(), last_name: "Doe".into() })
}
```

这将生成带版本的端点：

- `/api/v1/users/:id` → `get_user_v1`
- `/api/v2/users/:id` → `get_user_v2`

### 🛤️ 路径参数

遵循 Rust 命名规范提取路径参数。宏自动将路径段映射到函数参数：

```rust
#[forge(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    // `id` 自动从 `/users/:id` 提取
    Ok(User { id, name: "John".into() })
}
```

#### 🔹 多个路径参数

对于嵌套资源：

```rust
#[forge(
    name = "get_comment",
    version = "v1",
    path = "/posts/:post_id/comments/:comment_id",
    method = "GET"
)]
async fn get_comment(
    post_id: u64,
    comment_id: u64
) -> Result<Comment, ApiError> {
    // 两个参数均从路径提取
    Ok(Comment { post_id, comment_id, text: "Test".into() })
}

#[forge(
    name = "get_task",
    version = "v1",
    path = "/orgs/:org_id/projects/:project_id/tasks/:task_id",
    method = "GET"
)]
async fn get_task(
    org_id: u64,
    project_id: u64,
    task_id: u64
) -> Result<Task, ApiError> {
    Ok(Task { org_id, project_id, task_id, title: "Task".into() })
}
```

### ⚠️ 错误处理

定义自定义错误类型并转换为 `ServiceError`：

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Validation failed: {field}")]
    ValidationError { field: String },

    #[error("Unauthorized access")]
    Unauthorized,
}

impl From<MyError> for ServiceError {
    fn from(err: MyError) -> Self {
        match err {
            MyError::NotFound { resource } => ServiceError::with_details(
                "NOT_FOUND",
                format!("Resource not found: {}", resource),
                serde_json::json!({ "resource": resource }),
                404,
            ),
            MyError::ValidationError { field } => ServiceError::with_details(
                "VALIDATION_ERROR",
                format!("Validation failed for field: {}", field),
                serde_json::json!({ "field": field }),
                400,
            ),
            MyError::Unauthorized => ServiceError::new(
                "UNAUTHORIZED",
                "Authentication required",
                401,
            ),
        }
    }
}
```

### 🔧 `#[forge]` 宏参数

| 参数           | 说明                                                        | 必填 | 默认值 |
|----------------|-------------------------------------------------------------|------|--------|
| `name`         | 端点名称                                                    | 是   | -      |
| `version`      | API 版本                                                    | 是   | -      |
| `path`         | HTTP 路径（如 `/users/:id`）                                | 否   | -      |
| `method`       | HTTP 方法（GET/POST/PUT/DELETE 等）                         | 否   | GET    |
| `status`       | 显式声明成功状态码（如 201 用于 POST 创建）                 | 否   | 200    |
| `description`  | 端点描述                                                    | 否   | -      |
| `tool_name`    | MCP 工具名称                                                | 否   | -      |
| `grpc_method`  | gRPC 方法名（启用 `grpc` feature 时生效）                   | 否   | -      |
| `cli`          | 是否注册为 CLI 命令（启用 `cli` feature 时生效）            | 否   | false  |

### 🌐 协议组合

**仅 HTTP** — 传统 REST API：

```toml
[dependencies]
sdforge = { version = "0.5.0-rc.2", features = ["http"] }
```

**仅 MCP** — AI 工具集成：

```toml
[dependencies]
sdforge = { version = "0.5.0-rc.2", features = ["mcp"] }
```

**双协议** — 同一份代码同时通过 HTTP 与 MCP 暴露：

```toml
[dependencies]
sdforge = { version = "0.5.0-rc.2", features = ["http", "mcp"] }
```

**全量特性** — 启用全部能力：

```toml
[dependencies]
sdforge = { version = "0.5.0-rc.2", features = ["full"] }
```

### 🛰️ gRPC Dispatch

启用 `grpc` feature 后，`#[forge(grpc_method = "...")]` 会通过 inventory 注册到
`SdForgeGrpcService`，由其 `call()` 方法路由到对应 handler。返回值需满足
`serde::Serialize`，错误类型需为 `ApiError`：

```toml
[dependencies]
sdforge = { version = "0.5.0-rc.2", features = ["grpc"] }
```

```rust
use sdforge::prelude::*;
use sdforge::forge;

#[forge(
    name = "grpc_echo",
    version = "v1",
    grpc_method = "comprehensive.echo",
    description = "gRPC echo handler"
)]
async fn echo(msg: String) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({ "echo": msg }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    sdforge::init_all_plugins();
    let server = sdforge::grpc::SdForgeGrpcService::default();
    server.serve("0.0.0.0:50051").await?;
    Ok(())
}
```

### 🖥️ CLI Dispatch

启用 `cli` feature 后，`#[forge(cli = true)]` 会注册 `CliCommandRegistration` +
`CliHandlerRegistration`，由 `CliBuilder::execute()` 一站式完成 build / parse /
dispatch / 输出 / 退出。返回 `Value::String` 时输出原始串（不带引号），其他类型
输出 JSON：

```toml
[dependencies]
sdforge = { version = "0.5.0-rc.2", features = ["cli"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

```rust
use sdforge::cli::CliBuilder;
use sdforge::core::ApiError;
use sdforge::forge;

#[forge(name = "echo", version = "1.0", description = "Echo a greeting", cli = true)]
async fn echo(name: String) -> Result<String, ApiError> {
    Ok(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() {
    sdforge::init_all_plugins();
    // execute() 返回 `!`：内部 std::process::exit(0/1)，调用方无需 match
    CliBuilder::new().execute().await;
}
```

```sh
# 运行：cargo run --example basic_cli --features cli -- echo --name world
# 输出：Hello, world!   （无引号 —— 智能提取 Value::String）
```

---

## 🎨 特性标志

SDForge 使用 Cargo features 进行编译时协议选择和特性组合。

| 特性             | 描述                                     | 默认   |
|------------------|------------------------------------------|--------|
| `http`           | HTTP 服务器 (Axum 0.8)                   | ❌     |
| `mcp`            | MCP 协议 (rmcp 3.2, 2026-07-28 规范)     | ❌     |
| `streaming`      | SSE 流式传输支持                         | ❌     |
| `timestamp`      | 自动向响应添加时间戳                     | ❌     |
| `logging`        | 结构化请求日志                           | ❌     |
| `security`       | 安全特性 (认证, 限流, 审计)              | ❌     |
| `ratelimit`      | 限流核心 (基于 limiteron，不依赖 http)   | ❌     |
| `ratelimit-http` | HTTP 限流中间件 (Tower middleware)       | ❌     |
| `websocket`      | WebSocket 支持                           | ❌     |
| `grpc`           | gRPC 支持 (tonic)                        | ❌     |
| `cache`          | 缓存支持 (oxcache)                       | ❌     |
| `openapi`        | 自动 OpenAPI 3.1 规范生成                | ❌     |
| `cli`            | CLI 集成 (clap)                          | ❌     |
| `docs`           | 统一文档输出 (Swagger UI + Markdown)     | ❌     |
| `inklog`         | inklog 结构化日志集成                    | ❌     |
| `i18n`           | ICU4X 国际化 (本地化格式化)              | ❌     |
| `simd-json`      | SIMD 加速 JSON 序列化                    | ❌     |
| `full`           | 启用所有运行时特性                       | ❌     |

### 🔗 特性依赖关系

- `default`: 空（无预启用特性，需按需显式启用）
- `mcp`/`grpc`/`openapi`/`cli`/`streaming`/`cache`: 独立于 `http`
- `security`: 启用 `http`、`ratelimit-http`（含 `ratelimit`）与 `cache`，并引入 hmac/sha2/uuid 等安全依赖
- `ratelimit-http`: 需要 `http` + `ratelimit`
- `websocket`: 需要 `http` + `streaming`
- `docs`: 需要 `openapi` + `cli`（Swagger UI 子模块需额外启用 `http`）
- `kit`: trait-kit AsyncKit 集成，需要 `limiteron-integration` + `trait-kit`
- `full`: 启用全部运行时特性（不含 `simd-json` 与 `hex` 工具特性）

### 🔨 构建与测试

```bash
# 默认（无特性）
cargo build

# HTTP 协议
cargo build --features http

# MCP 协议
cargo build --features mcp

# 完整功能
cargo build --features full

# 自定义特性集
cargo build --features "http,cache,security"

# 测试
cargo test --features http
cargo test --features full

# 格式化与 Lint
cargo fmt
cargo clippy --all-features -- -D warnings
```

---

## 📚 文档

| 文档 | 说明 |
|------|------|
| [📖 用户指南](docs/USER_GUIDE.md) | 从安装到进阶的完整使用教程 |
| [📘 API 参考](docs/API_REFERENCE.md) | 全部公开 API 的详细说明 |
| [🏗️ 架构文档](docs/ARCHITECTURE.md) | 设计理念与内部实现 |
| [🔒 安全文档](docs/SECURITY.md) | 安全设计与最佳实践 |
| [⚡ 性能基准](docs/benchmarks/vs-server-less.md) | 特性门控 vs 全量打包的编译时间/体积对比 |
| [📋 更新日志](docs/CHANGELOG.md) | 每个版本的变更记录 |
| [🤝 贡献指南](docs/CONTRIBUTING.md) | 如何参与项目开发 |
| [📦 在线 API 文档](https://docs.rs/sdforge) | docs.rs 自动生成的最新文档 |

---

## 💻 示例

仓库包含两类示例。

### 可运行示例（根包 `cargo run --example`）

| 示例 | 所需特性 | 说明 |
|------|----------|------|
| `basic_cli` | `cli` | `#[forge(cli = true)]` + `CliBuilder` 一站式 CLI 入口 |
| `swagger_demo` | `docs` | Swagger UI 路由 + axum serve |
| `perf_regex_cache` | `cache` | 正则缓存性能验证 |
| `perf_lru_eviction` | `cache` | LRU 驱逐性能验证 |
| `perf_prefix_index` | `cache` | 前缀索引性能验证 |
| `perf_batch_ops` | `cache` | 批量操作性能验证 |

```bash
# CLI 示例
cargo run --example basic_cli --features cli -- echo --name world

# Swagger UI 示例
cargo run --example swagger_demo --features "docs http"

# 缓存性能示例
cargo run --example perf_regex_cache --features cache
```

### 综合示例库（workspace 成员 `sdforge-examples`，`examples/src/`）

| 模块 | 内容 |
|------|------|
| `basics/` | 简单 API、响应构建、类型与错误处理 |
| `http/` | 路由（路径参数、查询参数）、中间件（CORS） |
| `mcp/` | 工具定义与注册、MCP 2026-07-28 迁移（`migration_2026.rs`）、MRTR 会话（`mrtr_example.rs`） |
| `security/` | API Key 认证、认证失败场景、完整安全栈（`comprehensive.rs`） |
| `cache/` | 高级缓存模式（二级缓存、Cache-Aside、Write-Through） |
| `config/` | 配置管理（`app_config.rs`） |
| `streaming/` | SSE 流式响应 |
| `websocket/` | 基础用法与聊天室示例 |
| `grpc/` | gRPC 服务端 |
| `logging/` | 结构化日志 |
| `openapi/` | OpenAPI 规范生成（`OpenApiBuilder`、`generate_openapi_spec`） |
| `combined/` | 多特性组合的完整示例（`full_example.rs`） |

```bash
# 运行综合示例库的全部模块测试
cargo test --manifest-path examples/Cargo.toml --lib
```

示例配置文件位于 `examples/config/`（`default.toml`、`minimal.toml`、`production.toml`、`api-key-auth.toml`）。

---

## 🏗️ 架构

SDForge 采用「统一宏注解 → 编译期协议门控 → inventory 运行时注册」的架构。完整设计说明见 [架构文档](docs/ARCHITECTURE.md)。

```
sdforge/
├── src/                # 主框架 crate
│   ├── core/         # 核心类型、错误处理、验证
│   ├── error/        # 框架错误类型（ApiError、SdForgeError）
│   ├── http/         # HTTP 协议实现 (Axum)
│   ├── mcp/          # MCP 协议实现 (rmcp)
│   ├── security/     # 安全特性 (认证、限流、审计)
│   ├── cache/        # 缓存集成 (oxcache)
│   ├── websocket/    # WebSocket 支持
│   ├── grpc/         # gRPC 支持 (tonic)
│   ├── streaming/    # SSE 流式支持
│   ├── cli/          # CLI 集成 (clap)
│   ├── docs/         # 文档生成 (Swagger UI + Markdown)
│   ├── openapi/      # OpenAPI 3.1 规范生成
│   ├── domain/       # 领域抽象
│   ├── config/       # 配置管理
│   ├── i18n/         # 国际化 (ICU4X)
│   ├── integrations/ # trait-kit AsyncKit 集成
│   └── lib.rs        # 库入口点
├── macros/            # 过程宏 crate (#[forge])
├── examples/          # 综合示例库 (workspace member)
├── docs/              # 文档
├── benches/           # 基准测试
├── proto/             # protobuf 定义 (gRPC)
├── .github/           # GitHub 工作流
└── scripts/           # 构建和实用脚本
```

### 设计原则

- **编译时协议选择**：未使用的协议不产生任何编译代码
- **Inventory 注册模式**：`inventory::submit!()` 用于编译时注册，`init_all_plugins()` 防止链接器优化
- **三种构造模式**：所有组件支持 `new()`（开箱即用）、`builder()`（Builder 模式）、`with_dependencies()`（依赖注入）
- **不使用数据库**：所有数据交互通过 oxcache（内存缓存）完成

---

## 📜 OpenAPI 自动生成

SDForge 基于 [utoipa 5.5](https://crates.io/crates/utoipa) 自动生成 OpenAPI 3.1 规范。启用 `openapi` feature 后，每个 `#[forge]` 宏在编译期通过 `inventory` 注册 `OpenApiRouteInfo`；运行时调用 `generate_openapi_spec()` 收集全部路由并生成完整规范。

### 🔧 启用

```toml
[dependencies]
sdforge = { version = "0.5.0-rc.2", features = ["http", "openapi"] }
```

### 🚀 基本用法

```rust
use sdforge::openapi::generate_openapi_spec;

// 收集所有通过 #[forge] 注册的路由并生成 OpenAPI 规范
let spec = generate_openapi_spec();

// 序列化为 JSON 写入文件或返回给客户端
let json = serde_json::to_string_pretty(&spec).unwrap();
println!("{json}");
```

### 🎨 自定义元数据

使用 `OpenApiBuilder` 链式调用自定义 `info` 部分（title、version、description）。路由始终从全局 `inventory` 注册表收集：

```rust
use sdforge::openapi::OpenApiBuilder;

let spec = OpenApiBuilder::new()
    .title("My Service")
    .version("2.0.0")
    .description("User-facing API for the billing domain")
    .build();
```

### 🔗 宏集成

启用 `openapi` feature 后，`#[forge]` 自动生成注册代码，无需手动维护：

```rust
#[forge(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    description = "Get a user by ID"
)]
async fn get_user(id: u64) -> Result<User, ApiError> { /* ... */ }
```

上述代码会在编译期自动向全局注册表提交 `OpenApiRouteInfo { path: "/users/{id}", method: "GET", ... }`，`generate_openapi_spec()` 会将其纳入生成的规范。

> 注意：未启用 `openapi` feature 时，宏不生成任何 utoipa 相关代码——零运行时开销。

---

## 🔄 MCP 2026-07-28 迁移指南

v0.2.0 将 MCP 实现从 `mcp-sdk 0.0.3` 全面迁移至官方 [`rmcp`](https://crates.io/crates/rmcp) SDK（当前为 rmcp 3.2），适配 MCP 2026-07-28 规范。该迁移是一次 **BREAKING** 变更。

### ⚠️ BREAKING 变更

| 旧版本 (v0.1.x)                        | 新版本 (v0.2.0+)                              |
|-----------------------------------------|-----------------------------------------------|
| `mcp-sdk = "0.0"` 依赖                 | `rmcp` 依赖                                   |
| `initialize` 握手流程                   | 移除，改用 `server/discover` 端点             |
| 有状态会话 (`StatefulServerHandler`)    | 无状态适配层 (`StatelessServerHandler`)       |
| `register_mcp(&mut Server)` 签名        | `register_mcp(&mut dyn McpToolRegistry)`      |

### 🛠️ 无状态适配层

`StatelessServerHandler` 实现了 `rmcp::ServerHandler` trait，其方法均不依赖会话状态，适配 2026-07-28 规范的无状态协议模型：

```rust
use sdforge::mcp::stateless::StatelessServerHandler;

let handler = StatelessServerHandler::new();
// 通过 rmcp 的 axum 集成挂载到 HTTP 路由
```

### 📨 HTTP 头协议

无状态协议通过 HTTP 头传递方法名与工具名，由 `parse_mcp_headers` 解析：

```rust
use sdforge::mcp::headers::parse_mcp_headers;

// 客户端请求必须携带：
//   Mcp-Method: tools/call
//   Mcp-Name: get_user
let info = parse_mcp_headers(&headers)?;
```

缺少请求头返回 `400 Bad Request`，与 2026-07-28 规范一致。

### 🔁 多轮往返请求（MRTR）

新增 MRTR 支持。工具可通过 `InputRequiredResult` 挂起执行，等待客户端补充输入；300 秒超时后自动取消：

```rust
use sdforge::mcp::mrtr::MrtrSessionManager;

let manager = MrtrSessionManager::new();
let result = manager.create_session("session-1", "get_user")?;
// 客户端随后通过 session_id 恢复执行
```

### 💾 缓存语义

`cache_semantics` 模块处理 `ttlMs` 与 `cacheScope` 字段，支持 `global` 与 `request` 两种缓存作用域，并与 oxcache 集成实现工具结果缓存。

### 📚 迁移步骤

1. 将 `Cargo.toml` 中的 `mcp-sdk` 依赖替换为 `rmcp`
2. 将 `register_mcp(&mut Server)` 调用改为 `register_mcp(&mut dyn McpToolRegistry)`
3. 移除 `initialize` 握手相关代码，改用 `server/discover` 端点
4. 如需 MRTR 或缓存语义，导入对应模块

> 完整迁移示例见 `examples/src/mcp/migration_2026.rs`。

---

## 🚀 生产部署

### 🐳 Docker 部署

```dockerfile
FROM rust:1.85 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features full

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/sdforge /usr/local/bin/
EXPOSE 3000
CMD ["sdforge", "serve", "--port", "3000"]
```

### ☸️ Kubernetes 部署

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sdforge-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: sdforge-api
  template:
    metadata:
      labels:
        app: sdforge-api
    spec:
      containers:
      - name: sdforge
        image: sdforge:latest
        ports:
        - containerPort: 3000
        env:
        - name: FEATURES
          value: "full"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

### 🔧 环境配置

```bash
# 生产环境变量
export RUST_LOG=info
export SD_FORGE_PORT=3000
export SD_FORGE_HOST=0.0.0.0
export SD_FORGE_CONFIG_PATH=/etc/sdforge/config.toml
export SD_FORGE_FEATURES=full
```

---

## 🐛 故障排查

### 🔍 常见问题

#### **编译错误**

```bash
# 错误：找不到 feature
# 解决：检查可用的 features
cargo check --help | grep features

# 启用指定 features
cargo build --features "http,security,cache"
```

#### **运行时问题**

```bash
# 使用 tracing 查看日志
RUST_LOG=debug cargo run --features logging

# 端口冲突
# 解决：更换端口或结束占用进程
lsof -i :3000
kill -9 <PID>
```

#### **性能问题**

```bash
# 使用 cargo-flamegraph 剖析
cargo install flamegraph
cargo flamegraph --bin sdforge --features full

# 内存占用分析
valgrind --tool=massif target/release/sdforge
```

### 📋 健康检查端点

```rust
#[forge(
    name = "health_check",
    version = "v1",
    path = "/health",
    method = "GET"
)]
async fn health_check() -> Result<HealthStatus, ApiError> {
    Ok(HealthStatus {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: get_uptime(),
    })
}
```

### 🆘 获取帮助

- 📖 [文档](https://docs.rs/sdforge)
- 🐛 [Issue 跟踪](https://github.com/Kirky-X/sdforge/issues)
- 💬 [Discussions](https://github.com/Kirky-X/sdforge/discussions)

---

## 🧪 测试

### 测试分类

| 分类 | 位置 | 说明 |
|------|------|------|
| 单元测试 | `src/` 内嵌 `#[cfg(test)]`、`tests/unit/` | 模块级单元测试 |
| 集成测试 | `tests/integration/` | cache / config / error_handling / feature_combinations / grpc / http / mcp / openapi / security / status_code / streaming / uat / websocket / cli / docs |
| 宏测试 | `tests/macros/` | trybuild 编译失败用例与宏展开验证 |
| E2E 高级测试 | `tests/e2e_advanced.rs` | 覆盖 12 个模块未覆盖场景（178 个测试） |
| Examples 综合测试 | `examples/tests/comprehensive_features.rs` | 全 feature re-export 可访问性、跨协议 dispatch（77 个测试） |
| 基准测试 | `src/benches/` | criterion 基准（需 `http` feature） |

### 运行命令

```bash
# 按特性运行测试
cargo test --features http
cargo test --features mcp
cargo test --features "http,mcp"
cargo test --features full

# 仅运行 lib 测试（CI 覆盖率口径）
cargo test --features full --lib

# 运行指定测试
cargo test test_get_user --features http

# 带输出运行
cargo test --features http -- --nocapture

# Release 模式测试
cargo test --release --features http
```

CI 通过 `cargo llvm-cov --features full --lib` 生成覆盖率并上传 Codecov。

---

## 📊 性能

SDForge 的编译时特性门控带来显著的编译时间与产物体积优势（vs 全量打包 `--features full`）：

| 指标 | http only | full | 节省 |
|------|-----------|------|------|
| 编译时间（debug） | 28.88s | 54.52s | **47.0%** |
| 编译时间（release） | 13.72s | 25.48s | **46.2%** |
| 框架库体积 rlib（debug） | 33.9 MB | 100.2 MB | **66.2%** |
| 框架库体积 rlib（release） | 3.57 MB | 9.07 MB | **60.6%** |
| 唯一依赖 crate 数 | 396 | 478 | 17.2% |

> 数据来源：[docs/benchmarks/vs-server-less.md](docs/benchmarks/vs-server-less.md)（2026-07-03 实测，AMD Ryzen 9 9950X / rustc 1.93.1 / WSL2）。完整方法论、二进制体积分析与复现命令见该文档。

---

## 🔒 安全

SDForge 内置全套安全能力（`security` feature）：API Key / JWT Bearer 认证、限流（limiteron）、审计日志、安全头（CORS/CSP）、输入校验。概要设计与最佳实践见 [安全文档](docs/SECURITY.md)。

### 🛡️ API Key 认证

```rust
use sdforge::security::{ApiKeyAuth, auth_middleware};

let app = Router::new()
    .route("/api/*path", get(handler))
    .layer(auth_middleware(ApiKeyAuth::new("your-secret-key")));
```

### ⚡ 限流配置

```toml
# config.toml
[rate_limit]
enabled = true
requests_per_minute = 60
burst_size = 10
```

### ⚠️ 安全默认值（v0.3.0+）

> **注意**：v0.3.0 收紧了安全默认值，迁移时请检查：
> - **JWT 密钥最小长度**：`MIN_SECRET_LENGTH=32`，短于 32 字符的密钥将被拒绝
> - **ServerConfig 默认 host**：从 `"0.0.0.0"`（fail-open）改为 `"127.0.0.1"`（fail-safe 回环），生产部署必须显式配置 host
> - **CORS 校验收紧**：`"http://"`（仅 scheme 无 host）将被拒绝
>
> 另：v0.4.4 起 `extract_client_ip_core` 在无 `ConnectInfo` 时不再信任 `X-Forwarded-For` / `X-Real-IP` 头，生产部署**必须**配置 `ConnectInfo` 以启用不可伪造的 TCP 对端 IP 提取。

---

## 🗺️ 开发路线图

以下规划整理自 [CHANGELOG.md](docs/CHANGELOG.md) 未发布条目与工作区验收计划（ACCEPTANCE_PLAN.md）：

- **v0.5.0 发布（进行中）** — 当前处于 `0.5.0-rc.2`，按工作区验收计划完成依赖链（trait-kit/oxcache/inklog/limiteron）协同发布与终验
- **自定义成功状态码（已合入待发布）** — `#[forge(status = <code>)]` 静态声明 + `ServiceResponse::success_with_status` 动态控制（见 CHANGELOG [Unreleased]）
- **错误码行为契约统一** — 评估统一同一校验错误在 HTTP（400）与 gRPC（422）间的状态码差异（验收计划 SIMPL-001，现为记录在案的行为契约）
- **依赖治理** — 中期评估将 `bincode`（RUSTSEC-2025-0141 unmaintained）迁移至 `postcard` / `bitcode` / `rkyv`
- **MSRV 声明收敛** — 已按工作区 CONFIG_BASELINE 统一为 1.97.1（2026-09-06），覆盖 `--all-features` 下 1.94 的有效要求

---

## 🤝 参与贡献

我们欢迎贡献！请阅读 [贡献指南](docs/CONTRIBUTING.md) 了解开发环境、TDD 工作流和 PR 流程。

```bash
# 克隆仓库
git clone https://github.com/Kirky-X/sdforge.git
cd sdforge

# 安装 pre-commit 钩子
./scripts/install-pre-commit.sh

# 验证环境
cargo build --all-features
cargo test --all-features --lib
```

---

## 📋 更新日志

详见 [CHANGELOG.md](docs/CHANGELOG.md)。最近版本要点：

- **[Unreleased]** — `#[forge(status = <code>)]` 自定义成功状态码（静态声明 + `ServiceResponse::success_with_status` 动态控制，HTTP/gRPC 拉通，OpenAPI 同步）
- **[0.4.7]** — 依赖版本约束移除波浪号；补公开 `bincode` RUSTSEC-2025-0141 ignore 决策
- **[0.4.6]** — CI Clippy 修复；恢复 examples 的 `serde` 依赖
- **[0.4.5]** — 新增 `tests/e2e_advanced.rs`（178 个测试）

---

## 📄 许可证

本项目基于 MIT + Commons Clause 许可证发布，商业使用需单独授权。详见 [LICENSE](LICENSE)。

Copyright (c) 2026 Kirky.X

---

## 🙏 致谢

SDForge 站在优秀的开源生态之上，感谢以下项目：

- [Axum](https://github.com/tokio-rs/axum) / [Tower](https://github.com/tower-rs/tower) — HTTP 服务与中间件
- [rmcp](https://crates.io/crates/rmcp) — MCP 官方 Rust SDK
- [Tonic](https://github.com/hyperium/tonic) / [Prost](https://github.com/tokio-rs/prost) — gRPC 与 protobuf
- [utoipa](https://crates.io/crates/utoipa) — OpenAPI 规范生成
- [clap](https://github.com/clap-rs/clap) — 命令行解析
- [inventory](https://crates.io/crates/inventory) — 编译期注册
- [ICU4X](https://github.com/unicode-org/icu4x) — 国际化
- base 工作区姊妹项目 [oxcache](https://github.com/Kirky-X/oxcache)、[limiteron](https://github.com/Kirky-X/limiteron)、[trait-kit](https://github.com/Kirky-X/trait-kit)、[inklog](https://github.com/Kirky-X/inklog)

---

## 📞 联系与支持

- **🐛 Issue**：[github.com/Kirky-X/sdforge/issues](https://github.com/Kirky-X/sdforge/issues)
- **💬 讨论**：[github.com/Kirky-X/sdforge/discussions](https://github.com/Kirky-X/sdforge/discussions)
- **🏠 仓库**：<https://github.com/Kirky-X/sdforge>
- **📖 文档**：<https://docs.rs/sdforge>
- **👤 维护者**：Kirky.X

---

## ⭐ Star 历史

[![Star History Chart](https://api.star-history.com/svg?repos=Kirky-X/sdforge&type=Date)](https://star-history.com/#Kirky-X/sdforge&Date)

### 💝 支持本项目

如果您觉得这个项目有用，请考虑给它一个 ⭐️！

---

<div align="center">

**Built with ❤️ using Rust**

</div>

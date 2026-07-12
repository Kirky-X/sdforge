<div align="center" id="readme">

<img src="docs/asset/sdforge.png" alt="SDForge Logo" width="200" height="200">

[![CI](https://img.shields.io/github/actions/workflow/status/Kirky-X/sdforge/ci.yml?branch=main&label=CI)](https://github.com/Kirky-X/sdforge/actions) [![crates.io](https://img.shields.io/crates/v/sdforge.svg)](https://crates.io/crates/sdforge) [![docs.rs](https://img.shields.io/docsrs/sdforge.svg)](https://docs.rs/sdforge) [![downloads](https://img.shields.io/crates/d/sdforge.svg)](https://crates.io/crates/sdforge) [![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) [![rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

[English](./README_EN.md)

**SDForge** 是一个基于 Rust 的声明式 SDK 框架，利用过程宏从统一的函数注解自动生成多协议服务接口（HTTP + MCP + gRPC + WebSocket + CLI）。其核心创新在于通过 Cargo features 进行编译时协议选择——未使用的协议将产生零编译代码。

</div>

---

## ✨ 核心特性

- **🎯 统一接口定义** — 针对 HTTP、MCP、gRPC、WebSocket、CLI 的单一宏配置
- **⚡ 编译时协议选择** — 通过 Feature 控制代码生成，未使用的协议零运行时开销
- **🔒 类型安全** — 接口定义的编译时验证
- **🌐 多协议支持** — HTTP (Axum), MCP (rmcp 2.1), gRPC (tonic), WebSocket, SSE 流式传输, CLI (clap)
- **🧩 模块化设计** — 基于 Feature 的架构，允许仅选择所需功能
- **🛡️ 安全特性** — 内置认证（Bearer/API Key）、限流（limiteron）、审计日志
- **💾 缓存** — 基于内存缓存（oxcache 0.3.2），无需外部数据库
- **🔧 配置管理** — 自包含的 TOML 配置（无需外部配置中心）
- **📊 版本控制** — 内置 API 版本管理
- **📜 OpenAPI 自动生成** — 基于 utoipa 5.5 生成 OpenAPI 3.1 规范
- **🌐 国际化** — 基于 ICU4X 2.x 的本地化支持（i18n feature）

## 📦 快速开始

### 安装

```bash
cargo add sdforge
```

或手动添加到 `Cargo.toml`：

```toml
[dependencies]
sdforge = { version = "0.3", features = ["http"] }
```

### 基础使用

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

### 模块前缀

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
}
// 端点: /auth/api/v1/login
```

## 🔧 特性标志

SDForge 使用 Cargo features 进行编译时协议选择和特性组合。

| 特性         | 描述                                     | 默认   |
|--------------|------------------------------------------|--------|
| `http`       | HTTP 服务器 (Axum 0.8.9)                 | ✅     |
| `mcp`        | MCP 协议 (rmcp 2.1, 2026-07-28 规范)     | ❌     |
| `streaming`  | SSE 流式传输支持                         | ❌     |
| `timestamp`  | 自动向响应添加时间戳                     | ❌     |
| `logging`    | 结构化请求日志                           | ❌     |
| `security`   | 安全特性 (认证, 限流, 审计)              | ❌     |
| `ratelimit`  | 限流 (基于 limiteron 0.2.1)              | ❌     |
| `websocket`  | WebSocket 支持                           | ❌     |
| `grpc`       | gRPC 支持 (tonic)                        | ❌     |
| `cache`      | 缓存支持 (oxcache)                       | ❌     |
| `openapi`    | 自动 OpenAPI 3.1 规范生成                | ❌     |
| `cli`        | CLI 集成 (clap 4.6)                      | ❌     |
| `docs`       | 统一文档输出 (Swagger UI + Markdown)     | ❌     |
| `inklog`     | inklog 结构化日志集成                    | ❌     |
| `i18n`       | ICU4X 国际化 (本地化格式化)              | ❌     |
| `simd-json`  | SIMD 加速 JSON 序列化                    | ❌     |
| `full`       | 启用所有运行时特性                       | ❌     |

### 特性依赖关系

- `default`: [`http`]
- `mcp`/`grpc`/`openapi`/`cli`/`streaming`/`cache`: 独立于 `http`
- `security`: 需要 `http`、`cache`、`ratelimit`
- `websocket`: 需要 `http` + `streaming`
- `docs`: 需要 `openapi` + `cli`（Swagger UI 子模块需额外启用 `http`）

### 构建与测试

```bash
# 默认 (HTTP)
cargo build

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

## 🏗️ 架构

```
sdforge/
├── src/                # 主框架 crate
│   ├── core/         # 核心类型、错误处理、验证
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
│   └── lib.rs        # 库入口点
├── macros/            # 过程宏 crate (#[forge])
├── examples/          # 示例 (workspace member)
├── docs/              # 文档
├── .github/           # GitHub 工作流
└── scripts/           # 构建和实用脚本
```

### 设计原则

- **编译时协议选择**：未使用的协议不产生任何编译代码
- **Inventory 注册模式**：`inventory::submit!()` 用于编译时注册，`init_all_plugins()` 防止链接器优化
- **三种构造模式**：所有组件支持 `new()`（开箱即用）、`builder()`（Builder 模式）、`with_dependencies()`（依赖注入）
- **不使用数据库**：所有数据交互通过 oxcache（内存缓存）完成

## 📚 文档

- [📖 API 文档](https://docs.rs/sdforge)
- [💡 示例](./examples/)
- [📋 更新日志](CHANGELOG.md)
- [🤝 贡献指南](CONTRIBUTING.md)
- [🤖 Agent 指南](AGENTS.md)

## 🤝 贡献

我们欢迎贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解开发环境、TDD 工作流和 PR 流程。

## 📋 更新日志

详见 [CHANGELOG.md](CHANGELOG.md)。

## 📄 许可证

MIT License, Copyright (c) 2026 Kirky.X

详见 [LICENSE](LICENSE)。

---

<div align="center">

**Built with ❤️ using Rust**

[🔝 返回顶部](#readme)

</div>

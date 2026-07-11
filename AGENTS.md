# Agents Guide

**Project:** sdforge 0.3.3
**License:** MIT
**Edition:** 2024 (rust-version 1.91)
**Repository:** https://github.com/Kirky-X/sdforge

## Overview

SDForge 是一个使用过程宏自动生成多协议服务接口（HTTP + MCP + gRPC + WebSocket + CLI）的 Rust 声明式 SDK 框架。核心创新是编译时协议选择——未使用的协议产生零编译代码。

## Project Structure

```
sdforge/
├── src/                          # 主框架 (175 files, high complexity)
│   ├── core/                     # 核心类型、错误处理、验证
│   ├── http/                     # Axum HTTP 协议实现
│   ├── mcp/                      # MCP 协议实现 (rmcp 2.1)
│   ├── security/                 # 认证、速率限制、API Key
│   ├── cache/                    # Oxcache 缓存集成
│   ├── websocket/                # WebSocket 支持
│   ├── streaming/                # SSE 流式支持
│   ├── grpc/                     # gRPC 支持 (tonic)
│   ├── cli/                      # CLI 协议（clap 集成）
│   ├── docs/                     # 文档输出（Swagger UI + Markdown）
│   ├── openapi/                  # OpenAPI 3.1 规范生成
│   ├── domain/                   # 领域抽象
│   ├── config/                   # 配置管理
│   └── lib.rs                    # 库入口
├── macros/                       # 过程宏 crate (#[service_api])
│   ├── src/lib.rs                # 宏实现
│   └── tests/                    # trybuild 测试
├── examples/                     # 示例 (workspace member)
├── openspec/                     # OpenSpec 变更管理（历史 specs）
├── specmark/                     # specmark 规格驱动变更工作流（活跃）
├── docs/                         # 文档
├── scripts/                      # 脚本
├── rustfmt.toml                  # 格式化: 4空格, max_width=100
└── Cargo.toml                    # Workspace 配置
```

## Where to Look

| 任务 | 位置 | 说明 |
|------|------|------|
| 添加新 API | `src/lib.rs` | 定义宏和模块导出 |
| HTTP 路由 | `src/http/mod.rs` | Axum 路由构建 |
| MCP 工具 | `src/mcp/mod.rs` | MCP 工具注册 |
| CLI 命令 | `src/cli/mod.rs` | clap 集成，`CliCommandRegistration`/`CliHandlerRegistration` inventory 注册，`CliBuilder` 构造 `clap::Command`（builder.rs/handler.rs/docs_subcommand.rs） |
| 文档输出 | `src/docs/mod.rs` | `DocFormat` 枚举 + `generate_docs`/`write_docs` 入口，子模块 swagger.rs（Swagger UI Router）/cli_markdown.rs（clap-markdown）/mcp_markdown.rs（自定义） |
| 安全功能 | `src/security/` | 认证、授权、速率限制、审计（拆分为 `mod.rs` + 目录模块 `audit/`/`bearer/`/`types/`/`ratelimit/` + `ip_util.rs`/`middleware.rs`/`api_key.rs`/`api_key_manager.rs`/`traits.rs`，每个目录模块含 `mod.rs` + `tests/`；`ip_util.rs` 提供可信代理 IP 提取，被 auth 中间件与 ratelimit 适配器共享） |
| 缓存配置 | `src/cache/` | SyncCache/OxcacheSyncCache (mod.rs) |
| 流式响应 | `src/streaming/mod.rs` | SSE 实现 |
| WebSocket | `src/websocket/mod.rs` | WebSocketHandler |
| 配置加载 | `src/config/mod.rs` | 模块化配置（AppConfig/ServerConfig 等） |
| 宏定义 | `macros/src/lib.rs` | `#[service_api]` 解析（支持 `cli=true` 参数） |

## Conventions

- **edition 2024, rust-version 1.91+**
- **MIT License** — 所有源文件添加 `SPDX-License-Identifier: MIT` 头
- **依赖必须通过 feature 门控** — 所有可选依赖使用 `optional = true` 并在 `[features]` 中门控
- **不使用数据库** — 所有数据交互通过 oxcache（内存缓存）完成
- **混合模块风格** — 同时使用 `mod.rs` 和内联 `.rs` 文件

### 禁止
- 空 catch 块 `catch(e) {}`
- 删除失败的测试来"通过"
- 提交无法编译的代码
- 使用 `--no-verify` 绕过钩子

### 必须
- `Arc<dyn Trait>` 用于依赖注入
- trait 继承 `Send + Sync`
- 使用 `&self` 而非 `&mut self`
- 返回 `Option` 或 `Result`
- 实现 `Default` trait

### 构造模式（所有组件必须支持）

```rust
// 模式 1：开箱即用
let component = Component::new();

// 模式 2：Builder 模式
let component = Component::builder()
    .with_option(value)
    .build();

// 模式 3：完全依赖注入
let component = Component::with_dependencies(dep_a, dep_b);
```

### 特性门控

```bash
# 默认 (HTTP)
cargo build

# MCP 协议
cargo build --features mcp

# CLI 模式（clap 集成）
cargo build --features cli

# 文档输出（Swagger UI + Markdown，自动启用 cli + openapi）
cargo build --features docs

# 完整功能
cargo build --features full

```

### 错误处理
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Resource not found: {resource}")]
    NotFound { resource: String, resource_id: Option<String> },
    // ...
}
```

### 依赖注入
```rust
pub struct Component {
    cache: Arc<dyn CacheStore>,
    auth: Arc<dyn AuthService>,
}
```

### 协议注册
```rust
pub trait RouteRegistration: Send + Sync {
    fn register_http(&self, router: Router) -> Router;
    fn register_mcp(&self, server: &mut Server) -> Result<(), anyhow::Error>;
}
```

## Commands

```bash
# 构建所有特性
cargo build --all-features

# 测试（推荐使用 --lib 避免 macros trybuild 超时）
cargo test --all-features --lib

# 格式化
cargo fmt

# Clippy（零警告零错误）
cargo clippy --all-features -- -D warnings

# 文档
cargo doc --no-deps

# 基准测试
cargo bench --features http
```

## 测试分布

测试统一拆分到各模块的 `tests/` 子目录，每个测试文件聚焦单一关注点：

| 模块 | 测试数量 | 位置 |
|------|----------|------|
| security | 220+ | `src/security/{audit,bearer,types}/tests/`（audit_logger/builder/bearer_auth/types） |
| websocket | 30+ | `src/websocket/tests/`（mod/connection/handler/message） |
| core/error | 30+ | `src/core/error/tests/`（api_error/i18n） |
| core/str | 10+ | `src/core/str.rs`（`#[cfg(test)] mod tests`） |
| http | 70+ | `src/http/tests/`（routing/middleware/config） |
| grpc | 95+ | `src/grpc/tests/`（grpc_service/interceptor） |
| streaming | 75+ | `src/streaming/tests/`（sse/stream_builder） |
| mcp | 多项 | `src/mcp/tests/`（protocol/server/migration/handler） |
| ratelimit | 11+ | `src/security/ratelimit/tests/`（adapter/middleware/trait） |
| cli | 25+ | `src/cli/tests/`（trait_tests/builder_tests/handler_tests/macro_integration_tests/integration_tests/docs_subcommand_tests） |
| docs | 12+ | `src/docs/tests/`（mod_tests/swagger_tests/cli_markdown_tests/mcp_markdown_tests） |
| 集成测试 | - | `tests/integration/cli_tests.rs`、`tests/integration/docs_tests.rs`（端到端） |

## 复杂度热点

| 文件 | 行数 | 问题 |
|------|------|------|
| `src/core/validation.rs` | 1621 | 输入验证逻辑集中（已局部 `#[allow(clippy::result_large_err)]`） |
| `src/http/version_routing.rs` | 1366 | 版本路由 + 重定向逻辑 |
| `macros/src/lib.rs` | 1476 | 过程宏，多协议代码生成 + OpenAPI 路径参数自动映射 |
| `src/security/api_key_manager.rs` | 1010 | API Key 版本管理 + LRU |
| `src/security/api_key.rs` | 967 | API Key 生成/验证 |
| `src/security/middleware.rs` | 878 | 安全中间件链 |
| `src/security/audit/mod.rs` | 764 | 审计日志（已拆分 `tests/`） |
| `src/security/bearer/mod.rs` | 748 | Bearer 认证（已拆分 `tests/`） |
| `src/core/registration.rs` | 704 | 统一注册系统 |
| `src/security/types/mod.rs` | 602 | 安全类型定义（已拆分 `tests/`） |
| `src/core/regex_cache.rs` | 509 | 正则缓存（LRU 驱逐已修复） |
| `src/http/response.rs` | 448 | 响应封装 |
| `src/http/mod.rs` | 435 | 路由构建（已拆分 `tests/`） |
| `src/config/` (10 文件) | 2111 | 配置类型拆分到 app/server/auth/cache/security/cors/timeout/api/defaults |

## 跨领域关注点

### Inventory 注册模式
- `inventory::submit!()` 用于编译时注册
- 五种注册类型：HTTP路由、MCP工具、WebSocket路由、gRPC路由、CLI命令（+配对的 CliHandlerRegistration 闭包）
- `init_all_plugins()` 防止链接器优化

### Feature 依赖图
```
default ──► http
http ──► axum, tower, tower-http, inventory, tokio, axum-extra, toml, regex, once_cell, validator, uuid, futures
mcp ──► rmcp, inventory, anyhow, dep:http（独立协议，不依赖 sdforge http feature）
streaming ──► dep:tokio + dep:tokio-stream + dep:futures-util + dep:chrono（独立于 http；impl IntoResponse 已有 #[cfg(feature = "http")] 门控）
timestamp ──► chrono
logging ──► chrono, tokio
security ──► http + cache, uuid, hmac, sha2, chrono, tokio, secrets, zeroize, subtle, once_cell, argon2, password-hash, rand, regex, oxcache/memory, bincode, hex, base64, ratelimit
ratelimit ──► http + dep:limiteron (limiteron/tower-middleware, limiteron/ban-manager, limiteron/quota-control, limiteron/circuit-breaker)
websocket ──► http + streaming + tokio-tungstenite, axum-extra, async-trait, chrono
grpc ──► dep:tonic + dep:prost + dep:tonic-prost + inventory（独立于 http）
openapi ──► dep:utoipa + inventory（独立于 http）
cache ──► dep:http, oxcache/memory, async-trait（独立，使用外部 http crate 类型而非 sdforge http feature）
cli ──► dep:clap + inventory（独立于 http；clap features: std/derive/help/usage/error-context/suggestions）
docs ──► openapi + dep:clap-markdown + cli（Swagger UI 子模块需额外启用 http feature；swagger.rs 有 #[cfg(feature = "http")] 门控）
simd-json ──► dep:simd-json
hex ──► dep:hex
full ──► http, mcp, streaming, timestamp, security, cache, websocket, grpc, logging, openapi, cli, docs（不含 simd-json/hex 等工具特性）
```

## 注意事项

1. **宏注册**：使用 `inventory::submit!` 注册路由和工具，需要调用 `init_all_plugins()` 确保链接器不优化掉注册代码
2. **特性依赖**：`websocket` 依赖 `http` + `streaming`；`cli`/`openapi`/`grpc`/`streaming` 已解耦，可独立于 `http` 编译；`docs` 的 Swagger UI 子模块需 `http`，其余子模块独立
3. **外部依赖**：`oxcache` 来自 crates.io（0.3.2，features = ["memory"]），不再使用相对路径依赖
4. **测试**：测试嵌入在源文件中 `#[cfg(test)] mod tests`


## OpenSpec

项目使用 OpenSpec 进行变更管理。涉及规划、提案、架构变更时，**必须**阅读 `@/openspec/AGENTS.md`。

# SDForge 数据存储策略

## 项目定位

SDForge 项目属于**功能组件层 (Feature Layer)**，主要职责包括：

1. 提供高级业务功能（如日志、流控、监控、认证等）
2. 依赖底层组件层（oxcache 等基础设施组件）
3. 支持多种使用模式（开箱即用、Builder 模式、依赖注入）
4. 实现业务逻辑并对外暴露标准化接口
5. **重要：本项目不使用数据库，所有数据交互均通过 oxcache（内存缓存）完成**

## 依赖关系

- **底层组件层 (Infrastructure Layer)**：oxcache（缓存）等
- **功能组件层 (Feature Layer)**：本项目（SDForge）
- **应用层 (Application Layer)**：使用 SDForge 的最终应用程序

## 数据存储策略

- **不使用数据库**：项目明确不使用任何关系型或非关系型数据库
- **内存缓存**：使用 oxcache 进行临时数据存储和缓存
- **持久化**：如需持久化数据，依赖应用层实现

## 配置管理策略

### 自包含配置（无外部配置中心）

SDForge v0.3.0 移除了外部配置中心集成，采用**自包含**配置策略：

1. **手动 Default 实现**：所有配置类型通过手动 `impl Default` 或 `#[derive(Default)]` 提供默认值
2. **Builder 模式**：`AppConfigBuilder` 提供链式构造，未设置字段使用默认值
3. **TOML 加载**：通过 `toml` crate 从文件加载配置（`http` 特性启用）
4. **无外部依赖**：不再依赖外部配置中心，用户如需配置中心可在自身代码中集成

### 配置加载

```rust
use sdforge::config::AppConfig;

// 使用 Default 特性
let config = AppConfig::default();

// 或使用 Builder 模式
let config = AppConfig::builder()
    .server(server_config)
    .build();
```

### 配置类型

| 类型 | 说明 |
|------|------|
| `AppConfig` | 应用主配置，使用 `#[derive(Default)]` |
| `ServerConfig` | 服务器配置（host、port、timeout 等，默认 host=127.0.0.1） |
| `AuthConfig` | 认证配置（API Key、JWT 等，密钥最小 32 字符） |
| `CacheConfig` | 缓存配置（`default_ttl_secs`/`max_items`/`track_stats`） |
| `SecurityConfig` | 安全配置（CORS、headers、可信代理等） |
| `CorsConfig` | CORS 配置 |
| `TimeoutConfig` | 超时配置 |
| `ApiConfig` | API 配置 |

> **注**：本项目不使用数据库，故无 `DatabaseConfig`。所有数据交互通过 oxcache（内存缓存）完成。

## 组件实现规范

### 1. 三种构造模式

所有功能组件必须支持以下三种构造模式：

- **模式 1：开箱即用** - `new()` 方法，内部创建默认依赖
- **模式 2：Builder 模式** - `builder()` 方法，支持部分依赖注入
- **模式 3：完全依赖注入** - `with_dependencies()` 方法，由调用方提供所有依赖

### 2. 依赖注入规范

- 使用 `Arc<dyn Trait>` 类型作为依赖字段
- 不直接持有具体类型，而是通过 trait 接口依赖
- 所有 trait 必须继承 `Send + Sync` 以支持多线程

### 3. 与底层组件交互

- 通过预定义的 trait 接口与底层组件通信
- 不直接访问底层组件的具体实现
- 遵循接口隔离原则，仅使用必要的方法

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **sdforge** (5463 symbols, 13389 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/sdforge/context` | Codebase overview, check index freshness |
| `gitnexus://repo/sdforge/clusters` | All functional areas |
| `gitnexus://repo/sdforge/processes` | All execution flows |
| `gitnexus://repo/sdforge/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->

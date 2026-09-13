# 📖 Sdforge 用户指南

本指南面向 SDForge 的使用者，覆盖从安装、核心概念、配置到进阶用法的完整流程。SDForge 是一个基于 Rust 的声明式 SDK 框架，通过 `#[forge]` 过程宏从统一的函数注解自动生成多协议服务接口（HTTP + MCP + gRPC + WebSocket + CLI），并通过 Cargo features 进行编译时协议选择：未启用的协议产生零编译代码。

## 📋 目录

<details open>
<summary>📑 目录</summary>

- [简介](#-简介)
- [快速开始](#-快速开始)
- [核心概念](#-核心概念)
- [配置](#️-配置)
- [进阶用法](#-进阶用法)
- [最佳实践](#-最佳实践)
- [故障排查](#️-故障排查)

</details>

## 🧭 简介

SDForge 的核心思路是：**一份函数注解，多协议消费**。你只需要用 `#[forge]` 宏描述端点（名称、版本、路径、方法等），框架就会在编译期生成对应协议的注册代码：

- 启用 `http` → Axum 路由
- 启用 `mcp` → MCP 工具（官方 rmcp SDK，2026-07-28 规范）
- 启用 `grpc` → tonic gRPC 方法
- 启用 `cli` → clap 命令
- 启用 `openapi` → OpenAPI 3.1 规范条目

未启用的协议完全不进入编译产物，这是 SDForge 与"全量打包"框架的根本区别（量化对比见[编译期门控基准](benchmarks/vs-server-less.md)）。

## 🚀 快速开始

### 安装

```bash
cargo add sdforge
```

或手动添加到 `Cargo.toml`（当前版本 `0.5.0-rc.3`）：

```toml
[dependencies]
sdforge = { version = "0.5.0-rc.3", features = ["http"] }
```

> `sdforge` 默认不启用任何特性（`default = []`），需按需显式启用。

### 定义第一个 API

```rust
use sdforge::prelude::*;

#[forge(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    description = "Get a user by ID"
)]
async fn get_user(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({ "id": id, "name": "Test" }))
}

#[tokio::main]
async fn main() {
    sdforge::init_all_plugins(); // 固化 inventory 注册项，防链接器剔除
    let app = sdforge::http::build();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    sdforge::axum::serve(listener, app).await.unwrap();
}
```

运行后即可访问 `GET /api/v1/users/42`（版本会自动拼入路径前缀 `/api/{version}`）。示例中的 `sdforge::axum` 是框架转发的 axum 门面，下游无需直接依赖 axum。

## 🧩 核心概念

### `#[forge]` 宏

`#[forge]` 必填 `name` 与 `version`；`path` / `method` / `status` / `description` 等基础参数，以及 `i18n_key` / `cache_ttl` / `ws_path` / `stream` / `no_prefix` / `validate` / `paginate` / `on_start` / `on_stop` / `auth(role = "...")` 等进阶参数与裸旗标的完整列表、语义与默认值，见 [API 参考](API_REFERENCE.md#forge-参数) 的「`#[forge]` 参数」一节。

### 版本路由

`version` 参数决定路径前缀：`/api/{version}/...`。同一 `name` 可以并存多个版本，各自映射独立函数与响应类型。

### 模块前缀

`#[service_module(prefix = "/auth")]` 为模块内全部端点添加统一前缀，生成 `/auth/api/v1/login` 这类路径。

### 路径参数

路径中 `:id` 形式的段自动映射到同名函数参数，支持多级嵌套（如 `/posts/:post_id/comments/:comment_id`）。

### 统一注册（inventory）

`#[forge]` 生成的注册信息通过 `inventory::submit!()` 在编译期提交。应用启动时必须调用一次 `sdforge::init_all_plugins()`，防止链接器把注册项优化掉；它返回 `PluginCounts`，可用于确认各协议的注册数量。

### 统一 handler 契约

所有协议的 handler 遵循 `fn(HandlerArgs, HandlerState) -> HandlerFuture` 契约：`HandlerArgs` 自动从 clap / tonic / axum extractor 构造，应用状态通过 `downcast_state::<T>()` 注入（handler 中用 `#[state] db: Arc<Db>` 参数声明）。

## ⚙️ 配置

### 配置类型

配置模块（`sdforge::config`，需 `http` feature）提供 `AppConfig` / `ServerConfig` / `ApiConfig` / `AuthConfig` / `CorsConfig` / `TlsConfig` / `TracingConfig` / `CacheConfig` / `SecurityConfig` / `EnvHelper` / `ConfigError` 等类型；各类型的用途与默认值见 [API 参考](API_REFERENCE.md#️-配置扩展-api) 的配置模块一节。

### 使用配置构建

```rust
use sdforge::config::AppConfig;
use sdforge::http::build_with_config;

let config = AppConfig::default();
let app = build_with_config(&config)?;
```

### TOML 配置文件

SDForge 使用自包含的 TOML 配置（无需外部配置中心）。示例配置见仓库 `examples/config/`：`default.toml`、`minimal.toml`、`production.toml`、`api-key-auth.toml`。

限流与缓存配置示例：

```toml
# config.toml
[rate_limit]
enabled = true
requests_per_minute = 60
burst_size = 10

[cache]
enabled = true
default_ttl_secs = 600
max_items = 5000
track_stats = true
```

## 🔧 进阶用法

### gRPC 服务

启用 `grpc` feature 后，`#[forge(grpc_method = "...")]` 通过 inventory 注册 handler，由 `SdForgeGrpcService` 按 `grpc_method` 路由（实现 `Call` / `GetInfo` 两个 RPC）。服务器经 `build_server_with_config` 启动：

```rust
use sdforge::grpc::{GrpcServerConfig, build_server_with_config};
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
    let config = GrpcServerConfig::default();
    // security feature 启用时，require_auth 默认为 true：
    // 未配置 auth 服务器会拒绝启动（fail-safe），开发环境可显式置 false
    build_server_with_config("0.0.0.0:50051", config).await?;
    Ok(())
}
```

返回值需满足 `serde::Serialize`，错误类型需为 `ApiError`；参数载荷上限 1 MiB。应用状态经 `GrpcServerConfig.state` 注入（`Arc<dyn Any + Send + Sync>`）。

### CLI 应用

启用 `cli` feature 后，`#[forge(cli = true)]` 注册命令；`CliBuilder::execute()` 一站式完成构建/解析/分发/输出/退出。最小可运行示例见 [README 快速开始](../README.md#-最小可运行示例)（源码 `examples/basic_cli.rs`）。

返回 `Value::String` 时输出原始串（不带引号），其他类型输出 JSON。`CliBuilder` 还支持 `with_dependencies()`（注入状态）、`with_name()`（程序名）与 `with_global_arg()`（全局参数）。

### MCP 集成

- **无状态协议**：`StatelessServerHandler` 实现 `rmcp::ServerHandler`；HTTP 头协议由 `parse_mcp_headers` 解析 `Mcp-Method` / `Mcp-Name`（缺失返回 400）
- **stdio 服务**：`mcp::serve_stdio()` 封装 `rmcp::ServiceExt` + stdio 传输
- **MRTR 多轮往返**：`MrtrSessionManager` 管理会话，工具通过 `InputRequiredResult` 挂起等待补充输入，300 秒超时自动取消
- **缓存语义**：`cache_semantics` 模块处理 `ttlMs` / `cacheScope`（`global` / `request`）

迁移示例见 `examples/src/mcp/migration_2026.rs`。

### OpenAPI 文档

启用 `openapi` feature 后，`#[forge]` 自动注册 `OpenApiRouteInfo`；`generate_openapi_spec()` 与 `OpenApiBuilder` 的用法示例见 [API 参考](API_REFERENCE.md#openapi-生成)。

启用 `docs` feature 可进一步获得 Swagger UI（`swagger_ui_router()`，需 `http`）与 CLI/MCP Markdown 文档输出（`generate_docs` / `write_docs`）。

### SSE 流式与 WebSocket

- **SSE**（`streaming` feature）：`StreamEvent` / `StreamResponse` / `stream_to_sse` / `create_stream_channel`
- **WebSocket**（`websocket` feature，需 `http` + `streaming`）：`websocket_upgrade`、`WebSocketHandler`、`ConnectionManager`、`WebSocketConfig`

### 缓存

启用 `cache` feature 后直接透传 oxcache：`SyncCache` / `SharedCache` / `DashMapCache`（`OxcacheSyncCache` 别名），支持键规范化、模式失效（`invalidate(pattern)`）、批量删除（`delete_many`）与统计（`get_stats`）。HTTP 侧另有 `ResponseCacheLayer` 响应缓存中间件（需 `cache` + `http`），GET 路由成功响应自动缓存。

### 国际化与日志

- **i18n**：翻译注册表（`register_translation` / `set_locale` / `translate_or_fallback`）始终可用；`i18n` feature 额外提供 ICU4X 的 `HttpI18nFormatter`（本地化数字/日期/复数格式化与 Accept-Language 解析）
- **日志**：`logging` feature 提供 `StructuredLogger` / `init_global_logger`；`inklog` feature 将裸 `log` 调用桥接到 inklog `LoggerManager` 结构化管道（`init_inklog_logger()`）

## 💡 最佳实践

1. **按需启用特性** — 只需 HTTP 时用 `--features http`，不要默认上 `full`：编译时间节省约 46–47%、库体积缩减约六成（见[编译期门控基准](benchmarks/vs-server-less.md)）
2. **在 `main` 开头调用 `init_all_plugins()`** — 否则 release 构建（LTO + 死代码消除）可能剔除 inventory 注册项
3. **用 `From<MyError> for ServiceError` 统一错误** — 业务错误通过 `?` 自动转换，错误码/状态码集中管理
4. **安全相关部署实践以安全文档为准** — 生产 host、JWT 密钥 ≥ 32 字符、`ConnectInfo` 配置、API Key 显式播种与轮换等，见[安全文档](SECURITY.md#-安全最佳实践)
5. **为写操作声明状态码** — POST 创建类端点用 `#[forge(status = 201)]` 或 `ServiceResponse::success_with_status`
6. **参考示例代码** — `examples/src/` 按协议分模块，`security/comprehensive.rs` 是完整安全栈的参考实现

## 🛠️ 故障排查

| 现象 | 原因与解决 |
|------|------------|
| 注册的路由/工具/命令在运行时不存在 | 未调用 `init_all_plugins()`，或 release LTO 剔除了注册项；在 `main` 开头调用一次 |
| 编译错误：feature not found | 特性名拼写或组合错误；用 `cargo build --features "http,security,cache"` 显式列出；注意 `default = []` 不含任何协议 |
| 启用 `mcp` 或 `grpc` 后编译失败提到 `sdforge::http` | 旧版本遗留代码假设 `grpc` 依赖 `http`；当前版本协议互相独立，检查是否误写了过时的特性组合 |
| 服务启动后外部无法访问 | `ServerConfig` 默认绑定 `127.0.0.1`，生产环境需显式配置 host |
| 短 JWT 密钥被拒绝 | v0.3.0 起强制 `MIN_SECRET_LENGTH=32`，更换强密钥 |
| IP 限流/封禁不生效 | 未配置 `ConnectInfo`；v0.4.4 起无 `ConnectInfo` 时不再信任 `X-Forwarded-For` / `X-Real-IP` |
| MCP 请求返回 400 | 无状态协议要求请求头 `Mcp-Method` / `Mcp-Name`，缺失即 400（2026-07-28 规范行为） |
| gRPC 服务器拒绝启动并提示 authentication | 启用 `security` 时 `GrpcServerConfig.require_auth` 默认 `true`，需配置 `auth`（开发环境可显式置 `false`） |
| 端口冲突 | `lsof -i :3000` 查找占用进程后更换端口或结束进程 |
| 需要定位性能问题 | `cargo bench --bench runtime_bench --features http` 对照[性能基线](PERFORMANCE.md)；编译期成本对照[编译期门控基准](benchmarks/vs-server-less.md) |
| 其他问题 | 查阅 [API 参考](API_REFERENCE.md) 与 [架构文档](ARCHITECTURE.md)，或在 [Issues](https://github.com/Kirky-X/sdforge/issues) 提问 |

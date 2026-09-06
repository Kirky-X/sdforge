# 📘 Sdforge API 参考

本文档基于 `src/` 公开接口与 README 用法，汇总 SDForge 的公开 API。SDForge 的 API 面由两部分组成：始终可用的核心类型与宏，以及通过 Cargo features 门控的协议/能力模块——未启用的 feature 对应的 API 不参与编译，这正是"编译时协议选择"的落点。完整的类型签名与文档请以 [docs.rs/sdforge](https://docs.rs/sdforge) 为准。

## 📋 目录

<details open>

- [概述](#-概述)
- [核心 API](#-核心-api)
- [配置/扩展 API](#️-配置扩展-api)
- [错误类型](#-错误类型)
- [特性门控 API](#-特性门控-api)
- [使用示例](#-使用示例)
- [最佳实践](#-最佳实践)

</details>

## 🧭 概述

- **crate**：`sdforge`（运行时库）+ `sdforge-macros`（过程宏，经根 crate re-export）
- **统一入口**：绝大多数使用场景从 `use sdforge::prelude::*;` 开始，即可获得核心类型、宏以及 feature 对应的常用 re-export
- **依赖转发**：框架将宏生成代码所需的依赖以 re-export 形式转发（`sdforge::serde`、`sdforge::inventory`、`sdforge::axum`、`sdforge::rmcp`、`sdforge::tonic`、`sdforge::prost`、`sdforge::utoipa`、`sdforge::clap`、`sdforge::anyhow`、`sdforge::tokio_stream`、`sdforge::oxcache`、`sdforge::tower`、`sdforge::tower_http`），下游 crate 无需为使用 `#[forge]` 而直接依赖这些框架库

### 宏（无 feature 门控）

| 宏 | 说明 |
|----|------|
| `#[forge(...)]` | 统一端点注解，按启用的 feature 生成 HTTP / MCP / gRPC / WebSocket / CLI / OpenAPI 注册代码 |
| `#[service_module(prefix = "...")]` | 为模块内全部端点添加统一路径前缀 |
| `impl_default_new!(Type)` | 为单元结构体生成 `new()` 并实现 `Default` |

`#[forge]` 参数：`name`（必填）、`version`（必填）、`path`、`method`（默认 GET）、`status`（默认 200）、`description`、`tool_name`、`grpc_method`、`cli`。

## 🧱 核心 API

### `prelude` 模块

`sdforge::prelude` 导出：

- 核心类型：`ApiError`、`ApiMetadata`、`ServiceError`、`ServiceResponse`
- 宏：`forge`、`service_module`、`test_macro`
- `http` feature 下：`validate_email`、`validate_length`、`HttpRoute`、`RouteRegistration`、`IntoResponse`
- `mcp` feature 下：`McpToolInstance`
- `mcp` feature 下 re-export `anyhow`；`openapi` feature 下 re-export `utoipa`（供宏生成代码解析）

### 关键函数（多协议特性启用时）

| 函数 | 说明 |
|------|------|
| `init_all_plugins() -> PluginCounts` | 必须**至少调用一次**：固化 inventory 注册项防链接器剔除，返回各协议注册计数 |
| `get_websocket_routes()`（`websocket`） | 收集全部已注册 WebSocket 路由 |
| `get_mcp_tools()`（`mcp`） | 收集全部已注册 MCP 工具 |

`PluginCounts` 字段按 feature 条件编译：`routes`（HTTP）、`mcp_tools`、`ws_routes`、`grpc_routes`、`grpc_handlers`、`cli_commands`。

### 始终可用的其他 API

- `sdforge::error`：`SdForgeError`（统一错误枚举）、`SdForgeResult<T>` 别名、`ErrorContext`
- `sdforge::i18n`：`register_translation(locale, key, value)`、`set_locale`、`get_locale`、`translate_or_fallback(default, i18n_key)`、`clear_translations` —— 翻译注册表无需任何 feature，可对宏属性 `description` 做运行时多语言替换
- `sdforge::serde`：serde 的无条件 re-export（仅供类型引用；derive 宏仍需下游直接依赖 serde）

## ⚙️ 配置/扩展 API

配置模块 `sdforge::config`（`http` feature）：

| 类型/函数 | 说明 |
|-----------|------|
| `AppConfig` / `AppConfigBuilder` | 应用配置聚合根与 Builder |
| `ServerConfig` / `TlsConfig` / `TimeoutConfig` | 监听（默认 `127.0.0.1:8080`、30s 超时）、TLS、超时 |
| `ApiConfig` / `TracingConfig` / `EnvHelper` | API 行为、追踪配置、环境变量辅助 |
| `AuthConfig` / `ApiKeySeed` | 认证配置与 API Key 播种 |
| `CacheConfig` | 缓存配置（`enabled`、`default_ttl_secs`、`max_items`、`track_stats`） |
| `CorsConfig` / `build_cors_layer` | CORS 配置与层构建（校验非法 origin） |
| `SecurityConfig` / `defaults` | 安全配置与集中默认值 |
| `ConfigError` | 配置错误类型（实现 `ValidateConfig` 的类型另有 `validate()`） |

HTTP 构建入口 `sdforge::http`（`http` feature）：

| 函数/类型 | 说明 |
|-----------|------|
| `build() -> Router` | 从 inventory 注册项构建 Axum Router（开箱即用） |
| `build_with_config(&AppConfig) -> Result<Router, ConfigError>` | 按配置构建 |
| `build_with_redirect() -> Router` | 含重定向行为的构建 |
| `build_json_response` / `build_fallback_response` | JSON 响应与兜底响应构造 |
| `HttpRoute` / `RouteRegistration` | 路由描述与 inventory 注册项 |
| `VersionRouterConfig` / `VersionedRoute` / `build_version_router` | 版本路由 |
| `SecurityHeaders` | 安全响应头配置 |
| `rate_limit_layer`（`ratelimit-http`） | HTTP 限流层 |

扩展方式：自定义组件遵循三种构造模式 `new()` / `builder()` / `with_dependencies()`；协议扩展通过 `define_registration!` 宏与 `Registration` trait 接入统一注册系统。

## ❌ 错误类型

### `ApiError`（`sdforge::error::ApiError`，核心）

handler 返回类型 `Result<T, ApiError>` 的错误枚举（`serde` tagged、`thiserror` 派生）：

| 变体 | 字段 | 分类 |
|------|------|------|
| `NotFound` | `resource`、`resource_id: Option<String>` | ClientError |
| `InvalidInput` | `message`、`field`、`value` | ClientError |
| `AuthenticationFailed` | `reason` | AuthError |
| `AccessDenied` | `permission`、`user_id` | AuthError |
| `RateLimitExceeded` | `limit`、`window_seconds` | RateLimitError |
| `Internal` | `message`（脱敏）、`error_id`、`source`、`context: Option<Box<ErrorContext>>` | ServerError |
| `ServiceUnavailable` | `service`、`retry_after: Option<u64>`、`source` | ServerError |
| `ValidationError` | `field`、`constraint` | ValidationError |

常用方法：`category() -> ErrorCategory`（错误分类，供监控/告警）、`source()`（错误链）、`validation_error(code, message)` 构造器。`Internal` 的 `message` 经过清洗，不泄露内部实现细节。

### `ServiceError` / `ServiceResponse`（`sdforge::core`）

- `ServiceError::new(code, message, status)` / `ServiceError::with_details(code, message, details, status)` —— 业务错误统一载体，支持 `From<MyError>` 转换
- `ServiceResponse::success_with_status(data, code)` —— 动态指定成功状态码（与 `#[forge(status = ...)]` 对应）

### `SdForgeError`（框架统一错误）

`Api(ApiError)`、`Auth(AuthError)`、`Jwt(JwtError)`、`AuthConfig(AuthConfigError)`、`Config(ConfigError)`、`Internal(String)`；配套 `SdForgeResult<T>` 别名。

## 🚪 特性门控 API

| Feature | 模块 | 主要 API |
|---------|------|----------|
| `http` | `sdforge::http` / `config` / `axum`（facade） | `build`、`build_with_config`、`build_with_redirect`、`RouteRegistration`、`validate_email` / `validate_length`；axum/tower/tower-http re-export |
| `mcp` | `sdforge::mcp` | `SdForgeMcpServer`、`StatelessServerHandler`、`McpToolInstance` / `McpToolRegistration`、`build()`、`get_mcp_tools()`、`serve_stdio()`、`parse_mcp_headers` / `McpHeaderInfo`、`InputRequiredResult`、`MrtrSession` / `MrtrSessionManager`、`cache_semantics`；`rmcp` / `anyhow` re-export |
| `grpc` | `sdforge::grpc` | `SdForgeGrpcService`（`call()` 路由 / `serve()`）、`GrpcServerConfig`（含 `state: Option<Arc<dyn Any + Send + Sync>>`）、`build_server(_with_config)`、`GrpcRoute`、`CallRequest` / `CallResponse` / `InfoRequest` / `InfoResponse`、`SdForgeServiceServer`；`tonic` / `prost` re-export |
| `websocket` | `sdforge::websocket` | `WebSocketRoute` / `WebSocketHandler`、`websocket_upgrade` / `ValidatedWebSocketUpgrade`、`ConnectionManager`、`WebSocketConfig` / `WebSocketConnection` / `WebSocketMessage`、`parse_websocket_message` |
| `streaming` | `sdforge::streaming` | `StreamEvent`、`StreamResponse`、`stream_to_sse`、`create_stream_channel`；`tokio_stream` re-export |
| `security` / `ratelimit` / `ratelimit-http` | `sdforge::security` | 认证：`ApiKeyAuth`、`BearerAuth(+Builder)`、`AppApiKeyAuth(+Builder)`、`AuthContext`、`AuthExtractor`、`auth_middleware`；审计：`AuditLogger` / `AppAuditLogger(+Builder)`、`AuditLog` / `AuditResult`；限流：`RateLimiter` trait、`LimiteronAdapter`、`RateLimitLayer`（`ratelimit-http`） |
| `cache` | `sdforge::cache` | `Cache` / `CacheKey`、`SyncCache` / `SharedCache`、`DashMapCache`（`OxcacheSyncCache` 别名）；`oxcache` re-export |
| `openapi` | `sdforge::openapi` | `generate_openapi_spec()`、`OpenApiBuilder`（`title` / `version` / `description` / `build`）、`OpenApiRouteInfo` / `OpenApiPathParam`；`utoipa` re-export |
| `cli` | `sdforge::cli` | `CliBuilder`（`new`、`with_dependencies`、`with_name`、`with_global_arg`、`build -> clap::Command`、`execute -> !`）、`dispatch`、`GlobalArg`、`CliCommandRegistration` / `CliHandlerRegistration`；`clap` re-export |
| `docs` | `sdforge::docs` | `generate_docs` / `write_docs`、`DocFormat` / `DocError`；`swagger_ui_router`（另需 `http`） |
| `inklog` | `sdforge::inklog` | `init_inklog_logger()` —— 将 `log` 调用桥接到 inklog 结构化管道 |
| `i18n` | `sdforge::i18n` | `HttpI18nFormatter`（ICU4X 本地化格式化）、`I18nError` |
| `simd-json` | - | SIMD 加速 JSON 序列化路径 |
| `limiteron-integration` / `kit` | `sdforge::integrations` | trait-kit 0.3 AsyncKit 集成（`SdforgeModule`）、`LimiteronForgeAdapter` |

> 独立性说明：`mcp` / `grpc` / `openapi` / `cli` / `streaming` / `cache` 均独立于 `http`，可单独启用；`security` 拉入 `http` + `ratelimit-http` + `cache`；`websocket` 需 `http` + `streaming`；`docs` 需 `openapi` + `cli`。

## 💻 使用示例

### 最小 HTTP 服务

```rust
use sdforge::prelude::*;

#[forge(name = "get_user", version = "v1", path = "/users/:id", method = "GET")]
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

### 自定义错误转换

```rust
impl From<MyError> for ServiceError {
    fn from(err: MyError) -> Self {
        match err {
            MyError::NotFound { resource } => ServiceError::with_details(
                "NOT_FOUND",
                format!("Resource not found: {}", resource),
                serde_json::json!({ "resource": resource }),
                404,
            ),
            // ...
        }
    }
}
```

### OpenAPI 生成

```rust
use sdforge::openapi::{generate_openapi_spec, OpenApiBuilder};

let spec = generate_openapi_spec();                 // 收集全部 #[forge] 路由
let spec = OpenApiBuilder::new()
    .title("My Service")
    .version("2.0.0")
    .description("User-facing API")
    .build();
```

### CLI 一站式入口

```rust
use sdforge::cli::CliBuilder;

#[tokio::main]
async fn main() {
    sdforge::init_all_plugins();
    CliBuilder::new().execute().await; // 返回 `!`，内部 exit(0/1)
}
```

更多示例见仓库 `examples/`（运行方式见 [README 示例章节](../README.md#-示例)）。

## ✅ 最佳实践

1. **始终从 `prelude` 导入** — `use sdforge::prelude::*;` 保证宏生成代码引用的类型（`ApiError`、`utoipa`、`anyhow` 等）在下游可解析
2. **在 `main` 最早处调用 `init_all_plugins()`** — 它是幂等的（内部 OnceLock 缓存），返回 `PluginCounts` 可用于启动自检
3. **错误类型收敛到 `ApiError` / `ServiceError`** — 协议 handler 的错误约束是 `ApiError`（gRPC/CLI/HTTP 一致），业务错误通过 `From` 转换进入统一管道
4. **状态注入用 `with_dependencies` + `downcast_state`** — `CliBuilder::with_dependencies` / `GrpcServerConfig.state` 接收 `Arc<dyn Any + Send + Sync>`，handler 内通过 `#[state]` 参数与 `downcast_state::<T>()` 取回
5. **按 feature 最小化 API 面** — 未启用的 feature 对应模块不存在，宏也不生成对应代码；写库依赖时只声明实际用到的特性
6. **不要绕过框架 re-export** — 使用 `sdforge::axum::Router`、`sdforge::tonic::transport::Channel` 等转发路径，避免与直接依赖的版本冲突
7. **版本字段与路径参数保持命名一致** — 路径 `:id` 段依赖同名函数参数提取；`#[service_module]` 前缀与 `version` 共同决定最终路径

# 📘 Sdforge API 参考

本文档基于 `src/` 公开接口与 README 用法，汇总 SDForge 的公开 API。SDForge 的 API 面由两部分组成：始终可用的核心类型与宏，以及通过 Cargo features 门控的协议/能力模块：未启用的 feature 对应的 API 不参与编译，这正是"编译时协议选择"的落点。完整的类型签名与文档请以 [docs.rs/sdforge](https://docs.rs/sdforge) 为准。

## 📋 目录

<details open>
<summary>📑 目录</summary>

- [概述](#-概述)
- [核心 API](#-核心-api)
- [配置/扩展 API](#-配置扩展-api)
- [错误类型](#-错误类型)
- [特性门控 API](#-特性门控-api)
- [使用示例](#-使用示例)
- [最佳实践](#-最佳实践)
- [特性标志](#-特性标志)

</details>

## 🧭 概述

- **crate**：`sdforge`（运行时库）+ `sdforge-macros`（过程宏，经根 crate re-export）
- **统一入口**：绝大多数使用场景从 `use sdforge::prelude::*;` 开始，即可获得核心类型、宏以及 feature 对应的常用 re-export
- **依赖转发**：框架将宏生成代码所需的依赖以 re-export 形式转发（`sdforge::serde`、`sdforge::serde_json`、`sdforge::inventory`、`sdforge::axum`、`sdforge::rmcp`、`sdforge::tonic`、`sdforge::prost`、`sdforge::utoipa`、`sdforge::clap`、`sdforge::anyhow`、`sdforge::tokio_stream`、`sdforge::oxcache`、`sdforge::tower`、`sdforge::tower_http`），下游 crate 无需为使用 `#[forge]` 而直接依赖这些框架库

### 宏（无 feature 门控）

| 宏 | 说明 |
|----|------|
| `#[forge(...)]` | 统一端点注解，按启用的 feature 生成 HTTP / MCP / gRPC / WebSocket / CLI / OpenAPI 注册代码 |
| `#[service_module(prefix = "...")]` | 为模块内全部端点添加统一路径前缀 |
| `impl_default_new!(Type)` | 为单元结构体生成 `new()` 并实现 `Default` |
| `test_macro!` | 测试辅助：将函数包装为测试入口（随 prelude 导出） |

### `#[forge]` 参数

| 参数 | 说明 | 必填 | 默认值 |
|------|------|------|--------|
| `name` | 端点名称（不可为 Rust 保留字） | 是 | - |
| `version` | API 版本 | 是 | - |
| `path` | HTTP 路径（如 `/users/:id`） | 否 | - |
| `prefix` | 模块路径前缀：由 `#[service_module(prefix = "...")]` 注入模块内端点；显式声明可覆盖模块级注入 | 否 | - |
| `method` | HTTP 方法（GET/POST/PUT/DELETE 等） | 否 | GET |
| `status` | 成功状态码（100..=999） | 否 | 200 |
| `description` | 端点描述 | 否 | - |
| `i18n_key` | `description` 的运行时翻译键 | 否 | - |
| `deprecated` | 端点级弃用标记（裸旗标或 `= true/false`）：HTTP 成功响应附 `Deprecation: true` 头，gRPC 成功响应注入 `deprecation` metadata | 否 | false |
| `sunset` | 端点级退役日期/取值（HTTP 成功响应附 `Sunset` 头，gRPC 成功响应注入 `sunset` metadata） | 否 | - |
| `successor` | 后继端点提示（HTTP 成功响应附 `Link: <...>; rel="successor-version"`，gRPC 成功响应注入 `successor-version` metadata） | 否 | - |
| `tool_name` | MCP 工具名称（`mcp`） | 否 | - |
| `grpc_method` | gRPC 方法名（`grpc`） | 否 | - |
| `cli` | 注册为 CLI 命令（`cli`） | 否 | false |
| `cache_ttl` | 结果缓存 TTL 秒数（`cache`） | 否 | - |
| `ws_path` | WebSocket 挂载路径（`websocket`）：声明后 `websocket::build()` 挂载于该路径（缺少前导斜杠自动归一化）；未声明时派生 `/{name}` | 否 | - |
| `stream` / `streaming` | SSE 流式响应开关 | 否 | false |
| `no_prefix` | 跳过模块/版本前缀拼接 | 否 | false |
| `validate` | 裸旗标：使 `#[param(...)]` 的校验规则生效（`validate`），来源声明与 `flatten` 不受此旗标约束，属性表见下节 | 否 | false |
| `paginate` | 裸旗标：声明式分页（`paginate`） | 否 | false |
| `on_start` / `on_stop` | 裸旗标：进程生命周期钩子（`lifecycle`） | 否 | false |
| `auth(role = "...")` | 端点级 RBAC 角色（`http` / `mcp` / `grpc` 三协议生效：无匹配角色 HTTP 返回 403、gRPC 返回 `permission_denied`、MCP 返回 JSON-RPC `-32003`；CLI/WS 不生效） | 否 | - |

### `#[param]` 参数属性

handler 签名的每个参数经 `#[param(...)]` 声明提取来源与校验规则；规则的生效需端点带 `#[forge(validate)]` 旗标。

| 键 | 形态 | 说明 |
|------|------|------|
| `kind` | `kind = "..."` | 提取来源：`path` / `query` / `header` / `form` / `body` / `state` / `extension`（`#[state]` 属性等价 `kind = "state"`） |
| `ge` / `le` | `ge = 1, le = 100` | 数值下界 / 上界 |
| `min_length` / `max_length` | `min_length = 2, max_length = 10` | 字符串长度下界 / 上界 |
| `not_blank` | 裸标记 | 非空白校验 |
| `email` | 裸标记 | 邮箱格式校验 |
| `flatten` | 裸标记，或 `flatten = true` / `flatten = false` | **仅对 `kind = "query"` 成立**：结构体查询参数脱离信封，改由独立 `Query<ParamTy>` 槽扁平直提，handler 直接按字段取值（`filters.budget`，无 `.0`） |

#### 结构体查询参数（`flatten`）

`serde_urlencoded` 只支持扁平 `key=value`。结构体型 query 参数默认经信封 `Query<__ForgeQueryParams>` 提取，嵌套 struct 必然反序列化失败返回 400（该 fail-closed 行为由集成测试钉住）；显式 `flatten` 后该参数走独立提取槽：

```rust
#[derive(Debug, serde::Deserialize)]
struct SearchFilters {
    budget: u64,
    keyword: String,
}

#[forge(name = "search", version = "v1", path = "/search", method = "GET")]
async fn search(
    #[param(kind = "query", ge = 1)] page: u64,
    #[param(kind = "query", flatten)] filters: SearchFilters,
) -> serde_json::Value {
    // GET /api/v1/search?page=1&budget=50&keyword=hello
    serde_json::json!({ "page": page, "budget": filters.budget, "keyword": filters.keyword })
}
```

规则（宏展开期 fail-loud，不静默降级）：

- **与校验规则互斥**——被扁平化结构体的字段校验属于该结构体自身字段，`#[param(kind = "query", flatten, ge = 1)]` 编译报错；
- **只对 `kind = "query"` 成立**——其余 kind（含未声明 kind）带 `flatten` 编译报错；
- **布尔字面量限定**——只接受 `flatten`、`flatten = true`、`flatten = false`，非布尔值编译报错；
- **可多个共存**——同签名内多个 `flatten` 参数各得独立提取槽，并与信封提取的标量 query 参数互不冲突。

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

`PluginCounts` 六个字段恒存在（未启用对应协议时值恒 `0`），结构体本身门控于 `http` / `mcp` / `websocket` / `grpc` / `cli` 任一启用：`routes`（HTTP）、`mcp_tools`、`ws_routes`、`grpc_routes`、`grpc_handlers`、`cli_commands`。

### 始终可用的其他 API

- `sdforge::error`：`SdForgeError`（统一错误枚举）、`SdForgeResult<T>` 别名、`ErrorContext`
- `sdforge::i18n`：`register_translation(locale, key, value)`、`set_locale`、`get_locale`、`translate_or_fallback(default, i18n_key)`、`clear_translations`，翻译注册表无需任何 feature，可对宏属性 `description` 做运行时多语言替换
- `sdforge::serde` / `sdforge::serde_json`：无条件 re-export（仅供类型引用；derive 宏仍需下游直接依赖 serde）

## ⚙️ 配置/扩展 API

配置模块 `sdforge::config`（`http` feature）：

| 类型/函数 | 说明 |
|-----------|------|
| `SdForgeConfig` / `SdForgeConfigBuilder` | 应用配置聚合根与 Builder（含 `security` / `cache` 字段与 `build_rate_limiter()` 限流器构造——装配点见 `http::build_with_config_ext_rate_limited`） |
| `ServerConfig` / `TlsConfig` / `TimeoutConfig` | 监听（默认 `127.0.0.1:8080`、30s 超时）、TLS（`cert_path`/`key_path`/`alpn_protocols`，ALPN 缺省 `["h2", "http/1.1"]`）、超时 |
| `ApiConfig` / `TracingConfig` / `EnvHelper` | API 行为（路由前缀、默认版本）、追踪配置、运行环境名称辅助（`environment` 字段） |
| `AuthConfig` / `ApiKeySeed` | 认证配置与 API Key 播种（`AuthConfig::Jwt { secret, audience, issuer }`：`audience` / `issuer` 配置后令牌缺该声明或取值不符即拒绝） |
| `CacheConfig` | 缓存配置（`enabled`、`default_ttl_secs`、`max_items`、`track_stats`） |
| `SecurityConfig` / `defaults` | 安全响应头配置（CSP、X-Frame-Options 等；`enable_headers=false` 时不下发任何安全头）与集中默认值（`MIN_SECRET_LENGTH=32` 等） |
| `CorsConfig` / `build_cors_layer` | CORS 配置与层构建（校验非法 origin） |
| `ConfigError` | 配置错误类型（实现 `ValidateConfig` 的类型另有 `validate()`） |

HTTP 构建入口 `sdforge::http`（`http` feature）：

| 函数/类型 | 说明 |
|-----------|------|
| `build() -> Router` | 从 inventory 注册项构建 Axum Router（开箱即用） |
| `build_with_config_ext(Router, &SdForgeConfig) -> Result<Router, ConfigError>` | 按配置构建的唯一配置化入口：先把调用方路由并入再叠中间件——axum 的 `Router::layer` 只作用于调用它之前已注册的路由，先套栈后 merge 的路由会绕过鉴权，故不提供"先套栈"入口（自动装配中间件与 health/metrics 探针；**不装配限流层**）。探针路径归属：同名路径由调用方接管——inventory 注册的路由与调用方 `base` 声明的同名路径均优先，内置挂载跳过并告警（占用判定以「试注册」完成，axum 无路由枚举接口），不构建期 panic |
| `build_with_config_ext_rate_limited(Router, &SdForgeConfig) -> Result<Router, ConfigError>`（`ratelimit-http`） | 异步入口：既并入调用方路由（受鉴权/限流/安全头保护）又装上限流层（`security.rate_limit` 的实际生效点，限流在鉴权之外、探针与 `/metrics` 之外）——此前"并入路由"与"装限流"两个诉求互斥；探针路径归属同 `build_with_config_ext` |
| `build_with_redirect() -> Router` | 含重定向行为的构建 |
| `build_json_response` / `build_fallback_response` | JSON 响应与兜底响应构造 |
| `HttpRoute` / `RouteRegistration` | 路由描述与 inventory 注册项 |
| `VersionRouterConfig` / `VersionedRoute` / `build_version_router` | 版本路由 |
| `SecurityHeaders` | 安全响应头配置 |
| `rate_limit_layer`（`ratelimit-http`） | HTTP 限流层 |

TLS 终止 `sdforge::http::tls`（`serve-tls` feature，rustls aws-lc-rs；与 gRPC 侧 `grpc-tls` 的 `ServerTlsConfig` 接线实现独立）：

| 函数/类型 | 说明 |
|-----------|------|
| `load_server_config(&TlsConfig) -> Result<rustls::ServerConfig, TlsError>` | 加载 PEM 证书/密钥（装配期校验密钥匹配）并应用 ALPN |
| `tls_acceptor(&TlsConfig) -> Result<TlsAcceptor, TlsError>` | 静态证书 acceptor 便捷构建 |
| `ReloadingTls::new(&TlsConfig) -> Result<ReloadingTls, TlsError>` | 可热重载的 TLS 服务端（acceptor 与重载器编译期绑定）；`acceptor()` 取 serve 句柄，`reload()` 原子换入（阻塞，tokio worker 用 `reload_async()`），失败保留旧证书 |
| `serve_with_graceful_shutdown_tls(router, listener, acceptor, shutdown, TlsServeConfig)` | TLS 终止 serve：每请求注入 `ConnectInfo<SocketAddr>`（限流/审计取不可伪造 peer IP），复用 graceful 排空时序；accept 错误按 axum 语义退避重试不终止 |
| `TlsServeConfig` | serve 配置：`handshake_timeout`（默认 10s）/ `header_read_timeout`（默认 30s，仅 h1）/ `http2_keepalive_interval`（默认 30s）/ `http2_keepalive_timeout`（默认 20s）预认证护栏 + `graceful` 排空参数 |
| `TlsError` | 装配错误（文件读取 / PEM 解析 / rustls 配置 / 重载任务四类失败面） |

扩展方式：自定义组件遵循三种构造模式 `new()` / `builder()` / `with_dependencies()`；协议扩展通过 `define_registration!` 宏与 `Registration` trait 接入统一注册系统。

## ❌ 错误类型

### `ApiError`（`sdforge::error::ApiError`，核心）

handler 返回类型 `Result<T, ApiError>` 的错误枚举（`serde` tagged、`thiserror` 派生）：

| 变体 | 字段 | 分类 |
|------|------|------|
| `NotFound` | `resource`、`resource_id: Option<String>` | ClientError |
| `InvalidInput` | `message`、`field: Option<String>`、`value: Option<Value>` | ClientError |
| `AuthenticationFailed` | `reason` | AuthError |
| `AccessDenied` | `permission`、`user_id: Option<String>` | AuthError |
| `RateLimitExceeded` | `limit`、`window_seconds` | RateLimitError |
| `QuotaExhausted` | `used`、`total` | RateLimitError |
| `Internal` | `message`（脱敏）、`error_id`、`source`、`context: Option<Box<ErrorContext>>` | ServerError |
| `ServiceUnavailable` | `service`、`retry_after: Option<u64>`、`source` | ServerError |
| `ValidationError` | `field`、`constraint` | ValidationError |

常用方法：`category() -> ErrorCategory`（错误分类，供监控/告警）、`source()`（错误链）、`validation_error(code, message)` 构造器。`Internal` 的 `message` 经过清洗，不泄露内部实现细节。

### `ServiceError` / `ServiceResponse`（`sdforge::core`）

- `ServiceError::new(code, message, status)` / `ServiceError::with_details(code, message, details, status)`：业务错误统一载体，支持 `From<MyError>` 转换
- `ServiceResponse::success_with_status(data, code)`：动态指定成功状态码（与 `#[forge(status = ...)]` 对应）

### 统一错误契约（`sdforge::error::unified`，跨协议单一事实来源）

- `mapping_for(&ApiError) -> (u16, &'static str)`：`ApiError` → HTTP 状态码 + 机器错误码的唯一映射表（HTTP 适配器消费）
- `grpc_code_for(&ApiError) -> tonic::Code`（feature = `grpc`）：同一 `ApiError` → gRPC 状态码
- `grpc_code_for_http_status(u16) -> tonic::Code`（feature = `grpc`）：**跨协议行为契约表**，HTTP 状态 → gRPC 状态码的对齐轴（与 `code_for_http_status` 同行集）
- `code_for_http_status(u16) -> &'static str`：HTTP 状态 → 机器错误码
- `UnifiedError`：跨协议错误载荷 `{code, message, trace_id?, field?, details?}`，HTTP body 与 gRPC `Status::details` 共享同一形状；`details` 只在需要区分同状态码错误时出现（如 429 的 `RateLimitExceeded` / `QuotaExhausted`），`details.type` 取错误变体名（不本地化）

**错误码行为契约表**（同一 `ApiError` 在两条协议 wire 上的取值；由
`error_code_contract_tests` wire 级测试与 `unified` 单元测试全集 JOIN
不变量钉死，任一侧漂移即编译期后红灯）：

| `ApiError` 变体 | HTTP 状态 | 载荷 `code`（两协议共享） | gRPC 状态码 |
|-----------------|-----------|--------------------------|-------------|
| `InvalidInput` | 400 | `BAD_REQUEST` | `invalid_argument` |
| `ValidationError` | 422 | `UNPROCESSABLE_ENTITY` | `invalid_argument` |
| `NotFound` | 404 | `NOT_FOUND` | `not_found` |
| `AuthenticationFailed` | 401 | `UNAUTHORIZED` | `unauthenticated` |
| `AccessDenied` | 403 | `FORBIDDEN` | `permission_denied` |
| `RateLimitExceeded` | 429 | `TOO_MANY_REQUESTS` | `resource_exhausted` |
| `QuotaExhausted` | 429 | `TOO_MANY_REQUESTS` | `resource_exhausted` |
| `ServiceUnavailable` | 503 | `UNAVAILABLE` | `unavailable` |
| `Internal` | 500 | `INTERNAL` | `internal` |

语义约定：400 = 语法畸形/缺参/解析失败（`InvalidInput`）；422 = 语义约束违反
（`ValidationError` 及 `#[forge(validate)]` 违规，错误体含 `errors` 数组）。

429 家族的变体区分由载荷 `details.type` 承载（`RateLimitExceeded` / `QuotaExhausted`），
状态码与机器码仍是一状态一码：客户端据此决定退避重试还是停止重试，不解析本地化文案。

**单一词表**：本表是框架唯一的机器错误码词表。`ApiError::code()` 是其唯一
派生入口，HTTP 响应体（`UnifiedError`）、gRPC `Status::details`、
`ServiceError::code()` 与 MCP 输出（`to_mcp_json`）对同一错误给出同一个码
（`unified` 单元测试 `every_surface_reports_the_same_machine_code` 钉死）。
`SdForgeError` 的非 `Api` 变体同样从本表取值（`Auth` / `Jwt` → `UNAUTHORIZED`，
`AuthConfig` / `Internal` → `INTERNAL`，`Config` → `BAD_REQUEST`），错误子类型
由错误体的 `details.type` 承载（`auth` / `jwt` / `auth_config` / `config` /
`internal`），不另立机器码。
`ErrorCategory`（`error::context`）是监控用的粗分类，不是 wire 词表。

跨协议口径：gRPC 无与 HTTP 422 对应的原生状态码，400/422 在 gRPC 侧统一落
`invalid_argument`，语义区分由 HTTP 状态码与载荷 `code` 字段（gRPC 侧经
`Status::details` 携带）承载；幂等在途 409 → `already_exists`；契约表未登记
的状态 → `unknown` 兜底。

### 幂等防护（feature = `idempotency`）

- `cache::IdempotencyStore`：协议无关三态状态机（`Execute` / `InFlight` / `Replay(body, status)`），
  键前缀 `sdforge:idempotency:`，scope 绑定路由/gRPC method
- `http::idempotency::idempotency_middleware`：`Idempotency-Key` 头（POST/PUT/PATCH），
  重放响应附 `Idempotency-Replayed: true`，并发在途 409；`ServerConfig.idempotency`
  配置节（`enabled` 默认 false / `ttl_secs` 默认 86400 / `inflight_ttl_secs` 默认 30 /
  `max_response_bytes` 默认 1 MiB / `store`——外部注入的共享 `IdempotencyStore`，
  默认内部新建）
- gRPC：`idempotency-key` metadata，在途返回 `ALREADY_EXISTS`；经
  `GrpcServerConfig::idempotency_store` 注入

### `SdForgeError`（框架统一错误）

`Api(ApiError)`、`Auth(AuthError)`、`Jwt(JwtError)`、`AuthConfig(AuthConfigError)`、`Config(ConfigError)`、`Internal(String)`；配套 `SdForgeResult<T>` 别名。

`SdForgeError::to_service_error()` 的机器码与 HTTP 状态同源（经
`error::unified::code_for_http_status` 派生），与上表同一词表：

| 变体 | 状态 | `code` | `details.type` |
|------|------|--------|----------------|
| `Api` | 随 `ApiError` | 随 `ApiError` | 随 `ApiError` |
| `Auth` | 401 | `UNAUTHORIZED` | `auth` |
| `Jwt` | 401 | `UNAUTHORIZED` | `jwt` |
| `AuthConfig` | 500 | `INTERNAL` | `auth_config` |
| `Config` | 400 | `BAD_REQUEST` | `config` |
| `Internal` | 500 | `INTERNAL` | `internal` |

`Auth` / `Jwt` / `AuthConfig` 需 `security` 特性，`Config` 需 `http` 特性。

## 🚪 特性门控 API

| Feature | 模块 | 主要 API |
|---------|------|----------|
| `http` | `sdforge::http` / `config` / `axum`（facade）/ `rbac` | `build`、`build_with_config_ext`、`build_with_redirect`、`RouteRegistration`、`validate_email` / `validate_length`、`require_role`（配合 `#[forge(auth(role = "..."))]`）；axum/tower/tower-http re-export |
| `mcp` | `sdforge::mcp` | `SdForgeMcpServer`、`StatelessServerHandler`、`ToolCallContext`、`McpToolInstance` / `McpToolRegistration`、`build()`、`get_mcp_tools()`、`serve_stdio()`、`McpHeaderInfo`、`InputRequiredResult`、`MrtrSession`（会话管理器 `MrtrSessionManager` 不在根导出，需经 `sdforge::mcp::mrtr::MrtrSessionManager` 访问；头解析 `parse_mcp_headers` 同样不在根导出，需经 `sdforge::mcp::headers::parse_mcp_headers` 访问）、`cache_semantics`；`rmcp` / `anyhow` re-export |
| `grpc` | `sdforge::grpc` | `SdForgeGrpcService`（`Call` / `CallStream` / `GetInfo`；`CallStream` 需另启 `streaming`）、`GrpcServerConfig`（`state: Option<Arc<dyn Any + Send + Sync>>`、`require_auth`（默认 `true`）、`auth`（`security` 下为 `Option<BearerAuth>`，未启用时降级为 `Option<()>`）、`rate_limiter`、`extra_services: Vec<ExtraServiceMount>`（自定义 tonic service 同端口挂载回调；`Clone` 共享同一回调列表）、`max_connections`（默认 1000）、`timeout_seconds`（默认 30 秒）、`http2_keepalive_interval` / `http2_keepalive_timeout`（`None` = tonic 默认）、`tls`（`grpc-tls`，`Option<tonic::transport::ServerTlsConfig>`）、`idempotency_store` / `idempotency_ttl_secs`（默认 86400） / `idempotency_inflight_ttl_secs`（默认 30）（`idempotency`））、`build_server_with_config(addr, config)`、`build_server_with_graceful_shutdown(addr, config, signal)`（`signal: Future<Output = ()>`）、`build_server(addr)`（**已弃用**，见下方注记）、`GrpcRoute`、`CallRequest` / `CallResponse` / `InfoRequest` / `InfoResponse`、`SdForgeServiceServer`、`GrpcHandlerRegistration` / `GrpcStreamHandlerRegistration`（`streaming`）；`tonic` / `prost` re-export |
| `websocket` | `sdforge::websocket` | `WebSocketRoute` / `WebSocketHandler`、`websocket_upgrade` / `ValidatedWebSocketUpgrade`、`ConnectionManager`、`WebSocketConfig` / `WebSocketConnection` / `WebSocketMessage`、`parse_websocket_message` / `parse_websocket_message_with_limit`（后者按调用方给定的字节上限判定消息大小）、`build` / `build_with_config`（后者可注入带鉴权的 `WebSocketConfig`，握手校验 JWT / API key） |
| `streaming` | `sdforge::streaming` | `StreamEvent`、`StreamResponse`、`stream_to_sse`、`create_stream_channel`、`SseFrame` / `SseFrameParser` / `SseStreamError` / `frames`（入站 SSE 帧解析：WHATWG 分帧、`with_limits` 内存上限、零额外依赖）（`grpc_method` + `stream = true` 组合映射到 gRPC `CallStream`，见 `grpc` 行）；`tokio_stream` re-export |
| `ratelimit-dist` | `sdforge::security::ratelimit::dist` | `DistributedRateLimiter`（泛型 limiteron `DistributedLimiter` 后端：`InMemoryDistributedLimiter` 单实例/`RedisDistributedLimiter` 多副本）、`DistributedRateLimitConfig`（含适配层熔断 `with_circuit_failure_threshold`/`with_circuit_open_duration`：连续后端错误打开、打开期不经后端直接按策略裁决、半开探测恢复）、`BackendFailurePolicy`（fail-open 默认/fail-close） |
| `cache-l2` | `sdforge::cache::l2` | `RedisL2Cache`（oxcache `RedisBackend` 同步面 → `SyncCache`，仅多线程 runtime；异步面 `get_async`/`set_async`/`delete_async`/`contains_async` 供异步中间件热路径）、`RedisL2CacheConfig`（AUTH/TLS 连接串——Debug 输出掩码凭据、键前缀、默认 TTL）、`CacheL2Error` |
| `security` / `ratelimit` / `ratelimit-http` | `sdforge::security` | 认证：`ApiKeyAuth`、`BearerAuth(+Builder)`、`SdForgeApiKeyAuth(+Builder)`、`AuthContext`、`AuthExtractor`、`auth_middleware`；审计：`AuditLogger` / `SdForgeAuditLogger(+Builder)`、`AuditLog` / `AuditResult`、`AuditSink`；限流：`RateLimiter` trait、`LimiteronAdapter`、`RateLimitLayer`（`ratelimit-http`） |
| `cache` | `sdforge::cache` | `Cache` / `CacheKey`、`SyncCache` / `SharedCache`、`DashMapCache`（`OxcacheSyncCache` 别名）、`ResponseCacheLayer` / `ResponseCacheMiddleware` / `DEFAULT_MAX_CACHEABLE_BODY_BYTES` / `DEFAULT_CREDENTIAL_HEADER_NAMES`（另需 `http`；键 = 归一化 path + 原样 query；携带凭据的请求不进缓存，凭据头名经 `ResponseCacheLayer::with_credential_header_names` / `with_credential_headers` 或 `AuthConfig::credential_header_names()` 配置；超过可缓存上限的响应流式透传；带 `Set-Cookie` 的响应不进缓存）；`oxcache` re-export |
| `openapi` | `sdforge::openapi` | `generate_openapi_spec()`、`OpenApiBuilder`（`title` / `version` / `description` / `build`）、`OpenApiRouteInfo` / `OpenApiPathParam`；`utoipa` re-export |
| `schemars` | `sdforge::openapi`（反射 API） | `SchemaProbe::probe`、`PreciseSchema` / `FallbackSchema`（派生 `JsonSchema` 的返回类型产出字段级精确 response schema，未派生静默降级；蕴含 `openapi`） |
| `cli` | `sdforge::cli` | `CliBuilder`（`new`、`with_dependencies`、`with_name`、`with_global_arg`、`build -> clap::Command`、`execute -> !`）、`dispatch`、`GlobalArg`、`CliCommandRegistration` / `CliHandlerRegistration`；`clap` re-export |
| `docs` | `sdforge::docs` | `generate_docs` / `write_docs`、`DocFormat` / `DocError`；`swagger_ui_router`（另需 `http`） |
| `sdk` | `sdforge::sdk` | `generate_rust_client` / `generate_typescript_client`（纯函数渲染，`(method, path)` 确定性排序；Rust `Transport` 收完整 URL——`base_url` 由客户端拼接）、`ClientRoute` / `GrpcMethodInfo` / `collect_routes` / `collect_grpc_methods`；CLI 保留子命令 `sdk --lang ... --output-dir ... [--reqwest]` |
| `health` | `sdforge::health` | `CheckOutcome`（`healthy` / `unhealthy`）、`ReadinessCheck` / `HealthDataSource` trait、`register_readiness_check(_fn)` |
| `metrics` | `sdforge::metrics` | `MetricsRegistry`（`record` / `render`）、`global_registry()`、`record_request()`（Prometheus 文本格式 `/metrics`） |
| `context` | `sdforge::context` | `RequestContext`、`generate_id`、`scope`（request_id/trace_id 跨协议注入） |
| `lifecycle` | `sdforge::lifecycle` | `LifecycleHookRegistration`、`run_on_start` / `run_on_stop`（配合 `#[forge(on_start / on_stop)]`） |
| `hooks` | `sdforge::hooks` | `RequestHooks` trait、`install_hooks`、`hooks_middleware`（处理器前后钩子管道） |
| `otel` | `sdforge::otel` | `start_span` / `with_attr` / `finish_span` / `take_spans`、`OtelConfig`（OTLP/HTTP JSON 导出） |
| `inklog` | `sdforge::inklog` / `sdforge::forge::log` | `init_inklog_logger()`：将 `log` 调用桥接到 inklog 结构化管道；`#[forge::log]` 声明日志属性宏（`args`/`result`/`err_detail`/`level` 参数，DataMasker 脱敏 + 64 KiB 截断、级别守卫惰性渲染、`unsafe` 限定传播、`log_attr::mask`/`render_enter`/`render_exit` 可独立复用） |
| `i18n` | `sdforge::i18n` | `HttpI18nFormatter`（ICU4X 本地化格式化）、`I18nError`、`locale_from_accept_language` / `locale_from_headers`（请求语言解析）、`scope_request_locale` / `request_locale`（请求级 locale 作用域，`t()` 优先消费） |
| `limiteron-integration` / `kit` / `db-integration` / `db-permission` | `sdforge::integrations` | trait-kit AsyncKit 集成（`SdforgeModule`）、`LimiteronForgeAdapter`、dbnexus 数据 API 网关（`DbGateway`：`allow_table` + `query(GatewayQuery)` 白名单只读面、`GatewayQuery::all/filter/paging`、`with_session_role` 会话角色——仅在 `db-permission`（蕴含 `dbnexus/permission`）下被 dbnexus 逐表权限检查强制执行，否则只提供白名单与强制归属谓词隔离并显式告警） |

> 无独立模块的能力：`validate`（`#[forge(validate)]` + `#[param(...)]`，属性表见 [`#[param]` 参数属性](#param-参数属性)）、`paginate`（`#[forge(paginate)]`）、`etag`（GET 强 ETag + 304）、`graceful`（优雅停机）、`timestamp`（响应时间戳）、`simd-json`（SIMD JSON 路径）经宏旗标或构建配置生效。
>
> 独立性说明：`mcp` / `grpc` / `openapi` / `cli` / `streaming` / `cache` 均独立于 `http`，可单独启用；`security` 拉入 `http` + `ratelimit-http` + `cache`；`websocket` 需 `http` + `streaming`；`docs` 需 `openapi` + `cli`。
>
> **⚠️ `build_server(addr)` 已弃用**（源码标 `#[deprecated]`）：该入口启动**无认证**的 gRPC 服务器且无法配置认证，仅为向后兼容保留。新代码请用 `build_server_with_config(addr, config)` 并配置 `auth`——`require_auth` 默认 `true`，此时 `auth` 为 `None` 会在装配期拒绝启动（防误部无认证端点）。地址格式非法时两个入口均返回 `InvalidInput` 错误文本，不回显解析细节。

## 💻 使用示例

### 最小 HTTP 服务

`#[forge]` 注解 + `init_all_plugins()` + `http::build()` 的完整最小示例（含 Axum serve 与版本前缀说明）见 [用户指南](USER_GUIDE.md#-快速开始)。

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

`#[forge(cli = true)]` 注解与 `CliBuilder::execute()`（返回 `!`，内部 exit(0/1)）的最小示例见 [README 快速开始](../../README_CN.md#-最小可运行示例)，CLI 进阶用法见 [用户指南](USER_GUIDE.md#cli-应用)。

更多示例见仓库 `examples/`（完整清单与运行方式见 [用户指南 · 示例](USER_GUIDE.md#-示例)）。

## ✅ 最佳实践

1. **始终从 `prelude` 导入** — `use sdforge::prelude::*;` 保证宏生成代码引用的类型（`ApiError`、`utoipa`、`anyhow` 等）在下游可解析
2. **在 `main` 最早处调用 `init_all_plugins()`** — 它是幂等的（内部 OnceLock 缓存），返回 `PluginCounts` 可用于启动自检
3. **错误类型收敛到 `ApiError` / `ServiceError`** — 协议 handler 的错误约束是 `ApiError`（gRPC/CLI/HTTP 一致），业务错误通过 `From` 转换进入统一管道
4. **状态注入用 `with_dependencies` + `downcast_state`** — `CliBuilder::with_dependencies` / `GrpcServerConfig.state` 接收 `Arc<dyn Any + Send + Sync>`，handler 内通过 `#[state]` 参数与 `downcast_state::<T>()` 取回
5. **按 feature 最小化 API 面** — 未启用的 feature 对应模块不存在，宏也不生成对应代码；写库依赖时只声明实际用到的特性
6. **不要绕过框架 re-export** — 使用 `sdforge::axum::Router`、`sdforge::tonic::transport::Channel` 等转发路径，避免与直接依赖的版本冲突
7. **版本字段与路径参数保持命名一致** — 路径 `:id` 段依赖同名函数参数提取；`#[service_module]` 前缀与 `version` 共同决定最终路径

## 🎨 特性标志

`default = []`：所有特性均为可选，按需显式启用。

最低要求：

- **Rust 1.97.1+**（edition 2024，`rust-toolchain.toml` 固定工具链）
- **protoc**：仅在启用 `grpc` feature 时需要（`build.rs` 编译 protobuf）

<details>
<summary>📋 表格</summary>

<table>
  <tr><th>标志</th><th>说明</th><th>默认</th></tr>
  <tr><td><code>http</code></td><td>HTTP 服务器（Axum 0.8 路由、Tower 中间件、版本路由）</td><td>❌</td></tr>
  <tr><td><code>mcp</code></td><td>MCP 协议（rmcp 3.5，2026-07-28 规范：无状态 HTTP 头、MRTR、缓存语义）</td><td>❌</td></tr>
  <tr><td><code>mcp-http</code></td><td>MCP Streamable HTTP（启用 <code>rmcp/transport-streamable-http-server</code>）：挂载 rmcp 的 <code>StreamableHttpService</code> 于 <code>/mcp</code>——全部类型经 <code>sdforge::rmcp</code> 再导出解析，下游零直连 rmcp；挂载用 <code>sdforge::axum</code> 门面；已并入 <code>full</code></td><td>❌</td></tr>
  <tr><td><code>grpc</code></td><td>gRPC（tonic + prost，独立于 http，proto 经 build.rs 生成；unary <code>Call</code> + server-streaming <code>CallStream</code>——后者需另启 <code>streaming</code>；<code>extra_services</code> 支持应用自有 tonic service 同端口挂载）</td><td>❌</td></tr>
  <tr><td><code>grpc-tls</code></td><td>gRPC TLS 接线（暴露 tonic <code>ServerTlsConfig</code>，证书加载由调用方负责；与 HTTP 侧 <code>serve-tls</code> 的进程内终止实现独立，同样不并入 <code>full</code>）</td><td>❌</td></tr>
  <tr><td><code>websocket</code></td><td>WebSocket（依赖 http + streaming）</td><td>❌</td></tr>
  <tr><td><code>streaming</code></td><td>SSE 流式传输（独立于 http；与 <code>grpc</code> 组合时 <code>grpc_method</code> + <code>stream = true</code> 映射到 gRPC <code>CallStream</code>）</td><td>❌</td></tr>
  <tr><td><code>cli</code></td><td>CLI 集成（clap，独立于 http）</td><td>❌</td></tr>
  <tr><td><code>openapi</code></td><td>OpenAPI 3.1 规范生成（utoipa，独立于 http）</td><td>❌</td></tr>
  <tr><td><code>schemars</code></td><td>返回类型 Schema 反射（蕴含 openapi：<code>#[forge]</code> 为派生 <code>JsonSchema</code> 的返回类型生成字段级精确 response schema，未派生静默降级粗粒度映射）</td><td>❌</td></tr>
  <tr><td><code>docgen</code></td><td>统一文档输出（Swagger UI + CLI/MCP Markdown，依赖 openapi + cli；旧名 <code>docs</code> 为兼容别名。Swagger UI v5.32.6 资产自 vendor 于 <code>assets/swagger-ui/</code>，零额外依赖）</td><td>❌</td></tr>
  <tr><td><code>sdk</code></td><td>多协议客户端 SDK 生成（Rust 零依赖 / reqwest 可选 + TypeScript fetch+类型定义；保留 CLI 子命令 <code>sdk</code>，依赖 openapi + cli）</td><td>❌</td></tr>
  <tr><td><code>security</code></td><td>认证（API Key / JWT Bearer）、审计、安全头、限流与缓存（含 http + ratelimit-http + cache）</td><td>❌</td></tr>
  <tr><td><code>ratelimit</code></td><td>限流核心（limiteron，不依赖 http）</td><td>❌</td></tr>
  <tr><td><code>ratelimit-http</code></td><td>HTTP 限流中间件（Tower Layer，依赖 http + ratelimit）</td><td>❌</td></tr>
  <tr><td><code>ratelimit-dist</code></td><td>分布式限流（limiteron distributed 计数后端，多副本共享配额；后端不可达 fail-open/fail-close 显式可配，默认 fail-open）</td><td>❌</td></tr>
  <tr><td><code>cache</code></td><td>oxcache 内存缓存（独立于 http）</td><td>❌</td></tr>
  <tr><td><code>cache-l2</code></td><td>oxcache Redis L2 缓存（跨副本共享缓存层，蕴含 cache；故障固有 fail-open：读 miss/写跳过）</td><td>❌</td></tr>
  <tr><td><code>idempotency</code></td><td>幂等重放防护（<code>Idempotency-Key</code> HTTP 中间件 + gRPC metadata 路径；依赖 http + cache）</td><td>❌</td></tr>
  <tr><td><code>health</code></td><td><code>/healthz</code> <code>/readyz</code> 健康探针（<code>build_with_config_ext</code> 自动挂载，bypass 认证；inventory 注册或 <code>base</code> 声明的同名路由优先，内置探针跳过并告警，不 panic）</td><td>❌</td></tr>
  <tr><td><code>metrics</code></td><td>Prometheus 文本格式 <code>/metrics</code> 端点（请求计数、延迟直方图、状态码分布；自研轻量渲染；inventory 注册或 <code>base</code> 声明的同名路由优先，内置端点跳过并告警，不 panic）</td><td>❌</td></tr>
  <tr><td><code>graceful</code></td><td>优雅停机（SIGTERM/SIGINT：停止接新、排空在途、三阶段关闭）</td><td>❌</td></tr>
  <tr><td><code>serve-tls</code></td><td>HTTP TLS 终止（rustls aws-lc-rs：证书/密钥 PEM 加载、ALPN 可配置、<code>ReloadingTls</code> 热重载；每请求注入 <code>ConnectInfo</code>，握手/头读取超时护栏，停机编排与 graceful 共享收尾。与 gRPC 侧 <code>grpc-tls</code> 文档互链、实现独立）</td><td>❌</td></tr>
  <tr><td><code>tls-watch</code></td><td>TLS 证书去抖监听自动热重载（confers <code>FsWatcher</code> 监听 cert/key 文件变化驱动 <code>reload_async</code>；蕴含 <code>serve-tls</code>，重载失败保留旧证书）</td><td>❌</td></tr>
  <tr><td><code>context</code></td><td>请求上下文（request_id/trace_id 生成与跨协议 HTTP/MCP/gRPC/WS 注入）</td><td>❌</td></tr>
  <tr><td><code>validate</code></td><td><code>#[forge(validate)]</code> + <code>#[param(ge/le/...)]</code> 参数校验，422 返回字段级错误；结构体 query 参数经 <code>#[param(kind = "query", flatten)]</code> 扁平直提</td><td>❌</td></tr>
  <tr><td><code>paginate</code></td><td><code>#[forge(paginate)]</code> 声明式分页（自动 page/size 与 <code>{items,total,next}</code> 包装）</td><td>❌</td></tr>
  <tr><td><code>etag</code></td><td>ETag 条件请求（GET 响应自动附加 SHA-256 强 ETag，If-None-Match 返回 304）</td><td>❌</td></tr>
  <tr><td><code>lifecycle</code></td><td><code>#[forge(on_start/on_stop)]</code> 生命周期钩子（与优雅停机顺序协同）</td><td>❌</td></tr>
  <tr><td><code>hooks</code></td><td>处理器前后钩子管道（中间件式，统一错误契约）</td><td>❌</td></tr>
  <tr><td><code>otel</code></td><td>OTLP/HTTP JSON 导出请求 span 与指标快照（零额外依赖）</td><td>❌</td></tr>
  <tr><td><code>inklog</code></td><td>inklog 结构化日志桥接（<code>init_inklog_logger()</code>）+ <code>#[forge::log]</code> 声明日志属性宏（进入/退出/耗时/错误 + DataMasker 脱敏，feature 关闭时显性报错）。0.5.0-rc 起承接原自研 <code>StructuredLogger</code> 栈（<code>logging</code> feature 已删除，breaking）</td><td>❌</td></tr>
  <tr><td><code>timestamp</code></td><td>响应时间戳</td><td>❌</td></tr>
  <tr><td><code>i18n</code></td><td>ICU4X 国际化（本地化格式化 + Accept-Language 解析）</td><td>❌</td></tr>
  <tr><td><code>simd-json</code></td><td>SIMD 加速 JSON 序列化/反序列化</td><td>❌</td></tr>
  <tr><td><code>limiteron-integration</code></td><td>引入 limiteron 依赖（kit 集成基座）</td><td>❌</td></tr>
  <tr><td><code>db-integration</code></td><td>dbnexus 数据 API 网关（<code>DbGateway</code> 白名单只读数据面：表/列白名单 + 过滤值转义 + 服务端分页夹紧，独立于 http）</td><td>❌</td></tr>
  <tr><td><code>db-sqlite</code></td><td><code>db-integration</code> + dbnexus/sqlite 驱动（0.5.1 起 dbnexus 集成切 <code>embedded-runtime</code>，默认不含 sqlite；需 sqlite 文件库的网关用户显式启用）</td><td>❌</td></tr>
  <tr><td><code>db-permission</code></td><td>开启 dbnexus permission：逐表权限检查对非 admin 会话角色生效（fail-closed）；未开启时权限检查整块编译掉，仅剩白名单与归属谓词隔离</td><td>❌</td></tr>
  <tr><td><code>kit</code></td><td>trait-kit AsyncKit 集成（SdforgeModule 模块图）</td><td>❌</td></tr>
  <tr><td><code>tokio</code></td><td>内部特性：启用 tokio 依赖（随其他特性自动引入）</td><td>❌</td></tr>
  <tr><td><code>full</code></td><td>全部运行时特性（26 项：http/mcp/grpc/websocket/streaming/security/cache/health/metrics/graceful/context/validate/paginate/etag/hooks/lifecycle/otel/timestamp/openapi/cli/docgen/inklog/i18n/idempotency/serve-tls/tls-watch；不含 gRPC 侧 <code>grpc-tls</code> 与 <code>simd-json</code>/<code>kit</code>/<code>limiteron-integration</code>/<code>schemars</code>/<code>ratelimit-dist</code>/<code>cache-l2</code>/<code>db-integration</code>/<code>sdk</code> 可选重依赖与 opt-in 面）</td><td>❌</td></tr>
</table>

</details>

<details>
<summary>🔗 特性依赖关系</summary>

- 独立于 `http`：`mcp` / `grpc` / `openapi` / `cli` / `streaming` / `cache` / `timestamp` / `context` / `inklog` / `i18n` / `simd-json` / `limiteron-integration` / `db-integration`（含 `db-sqlite` / `db-permission` 派生） / `schemars` / `ratelimit-dist`
- **下游镜像 feature 的 check-cfg 片段**（宏发射代码按下游 crate 的同名
  feature 门控，见 examples 的 `[lints]` 白名单范式）：下游 `Cargo.toml`
  加 `[lints.rust] unexpected_cfgs = { level = "allow", check-cfg =
  ['cfg(feature, values("http", "mcp", "grpc", "cli", "websocket",
  "openapi", "security", "lifecycle"))'] }`（按实际启用的镜像 feature
  裁剪），避免未知 cfg 告警
- 派生自 `http`：`security`（含 `ratelimit-http` → `ratelimit` 与 `cache`）、`ratelimit-http`、`websocket`（含 `streaming`）、`health` / `metrics` / `graceful` / `serve-tls`（含 `graceful`）/ `tls-watch`（含 `serve-tls`）/ `validate` / `paginate` / `lifecycle` / `hooks` / `otel`
- `docgen` = `openapi` + `cli`（Swagger UI 挂载需另启用 `http`；旧名 `docs` 为兼容别名）
- `kit` = `trait-kit`（health + lifecycle）+ `limiteron-integration` + `limiteron/kit` + `oxcache/kit`
- `full` 覆盖 26 项运行时特性（含 `idempotency`、`docgen`、部署面 HTTP TLS `serve-tls`/`tls-watch`），不含 `simd-json` / `kit` / `limiteron-integration` 可选重依赖（SIMD JSON、trait-kit 模块图、限流集成基座）与 `grpc-tls`（gRPC 侧 TLS 接线）/ `schemars` / `ratelimit-dist` / `cache-l2` / `db-integration` / `sdk`（opt-in 面），均按需单独启用

> ℹ️ **`docgen` 可用性说明**（2026-10-16 上游 `zip` 全系 yank 事件）：该事件曾致 `docgen` 依赖的 `utoipa-swagger-ui`（锁定 `zip ^8.6`）无法完成依赖解析，全新启用 `docgen`（含 `full`，其覆盖 `docgen`）直接报错。现已移除该依赖：Swagger UI v5.32.6 官方 dist 功能子集（Apache-2.0，修改说明见资产 `NOTICE.md`）自 vendor 于 `assets/swagger-ui/` 并经 `include_bytes!` 编译期内嵌，`swagger_ui_router*` 直接服务；`docgen` 收缩为 `openapi + cli`，下游依赖解析恢复正常。升级资产经 `scripts/update_swagger_ui.sh`。

</details>

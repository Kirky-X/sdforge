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

版本固定的 `Cargo.toml` 写法与「`default = []` 需按需显式启用协议特性」的说明见 [README · 快速开始](../README.md#-快速开始)。

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

配置模块（`sdforge::config`，需 `http` feature）提供 `SdForgeConfig` / `ServerConfig` / `ApiConfig` / `AuthConfig` / `CorsConfig` / `TlsConfig` / `TracingConfig` / `CacheConfig` / `SecurityConfig` / `EnvHelper` / `ConfigError` 等类型；各类型的用途与默认值见 [API 参考](API_REFERENCE.md#️-配置扩展-api) 的配置模块一节。

### 使用配置构建

```rust
use sdforge::config::SdForgeConfig;
use sdforge::http::build_with_config;

let config = SdForgeConfig::default();
let app = build_with_config(&config)?;
```

### TOML 配置文件

SDForge 使用自包含的 TOML 配置（无需外部配置中心）。示例配置见仓库 `examples/config/`（完整清单见 [README · 示例](../README.md#-示例)）。

限流与缓存配置示例：

```toml
# config.toml
[security.rate_limit]
rate = 60            # 每窗口允许请求数（limiteron FlowControlConfig）
window_seconds = 1   # 窗口时长

[cache]
enabled = true
default_ttl_secs = 600
max_items = 5000
track_stats = true
```

## 🔧 进阶用法

### 参数校验与结构体查询参数（`validate` × `#[param]`）

handler 参数用 `#[param(...)]` 声明提取来源与规则，`kind` 取 `path` / `query` / `header` / `form` / `body` / `state` / `extension`（`#[state]` 属性等价 `kind = "state"`），规则有 `ge` / `le` / `min_length` / `max_length` / `not_blank` / `email`；规则生效需端点带 `#[forge(validate)]` 旗标，不通过返回 422 字段级错误。

`serde_urlencoded` 只支持扁平 `key=value`，结构体型 query 参数默认经信封提取会嵌套失败返回 400。要按扁平 query 接收结构体，显式 opt-in `flatten`：

```rust
#[derive(Debug, serde::Deserialize)]
struct SearchFilters {
    budget: u64,
    keyword: String,
}

#[forge(name = "search", version = "v1", path = "/search", method = "GET", validate)]
async fn search(
    #[param(kind = "query", ge = 1)] page: u64,
    #[param(kind = "query", flatten)] filters: SearchFilters,
) -> Result<serde_json::Value, ApiError> {
    // GET /api/v1/search?page=1&budget=50&keyword=hello
    Ok(serde_json::json!({ "page": page, "budget": filters.budget, "keyword": filters.keyword }))
}
```

`flatten` 的四条约束在宏展开期 fail-loud：与校验规则互斥（字段校验属于被扁平化结构体自身）、只对 `kind = "query"` 成立、只接受布尔字面量（`flatten` / `flatten = true` / `flatten = false`）、同签名内可多个共存且与信封提取的标量参数互不冲突。完整属性表与语义见 [API 参考 · `#[param]` 参数属性](API_REFERENCE.md#param-参数属性)。

### gRPC 服务

启用 `grpc` feature 后，`#[forge(grpc_method = "...")]` 通过 inventory 注册 handler，由 `SdForgeGrpcService` 按 `grpc_method` 路由（实现 `Call` / `GetInfo`，及 server-streaming `CallStream`——需启用 `streaming`，未启用时返回 unimplemented）。服务器经 `build_server_with_config` 启动：

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

> **⚠️ 不要用 `build_server(addr)`**：该入口已弃用（源码标 `#[deprecated]`），它启动的是**无认证**服务器且无法配置认证。统一走 `build_server_with_config`。

#### gRPC 优雅停机（`build_server_with_graceful_shutdown`）

装配链与 `build_server_with_config` 完全一致（认证拦截器、连接上限/超时/keepalive、TLS、`extra_services`），末尾走 tonic `serve_with_shutdown`：`signal` future 完成后停止接受新连接、等待 in-flight 请求完成后返回 `Ok(())`（语义对齐 HTTP 侧 axum `with_graceful_shutdown`：以 future 完成为准，永不完成的 future 即永驻）：

```rust
use sdforge::grpc::{GrpcServerConfig, build_server_with_graceful_shutdown};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    sdforge::init_all_plugins();
    let config = GrpcServerConfig::default();
    build_server_with_graceful_shutdown("0.0.0.0:50051", config, async {
        tokio::signal::ctrl_c().await.ok();
    })
    .await?;
    Ok(())
}
```

#### 服务参数

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `max_connections` | `usize` | 1000 | 并发连接上限 |
| `timeout_seconds` | `u64` | 30 | 单请求超时（秒）；0 为合法值（集成测试覆盖） |
| `http2_keepalive_interval` / `http2_keepalive_timeout` | `Option<Duration>` | `None` | HTTP/2 keepalive 探测间隔与超时（`None` = tonic 默认） |
| `require_auth` | `bool` | `true` | `true` 且 `auth` 为 `None` 时装配期拒启 |
| `auth` | `Option<BearerAuth>`（`security`）/ `Option<()>` | `None` | JWT 校验凭据；未启用 `security` 时该字段不可用 |
| `rate_limiter` | 限流器注入 | - | 服务端限流（`ratelimit`） |
| `extra_services` | `Vec<ExtraServiceMount>` | 空 | 应用自有 tonic service 同端口挂载（`Clone` 共享同一回调列表） |
| `tls` | `Option<tonic::transport::ServerTlsConfig>`（`grpc-tls`） | `None` | 只做接线，不做证书加载/轮换 |
| `idempotency_store` / `idempotency_ttl_secs` / `idempotency_inflight_ttl_secs` | （`idempotency`） | `None` / 86400 / 30 | 幂等重放防护 store、重放窗口与在途 claim 上限 |

#### gRPC server-streaming（`streaming` × `grpc`）

`#[forge(grpc_method, stream = true)]`（另需 `streaming` feature）声明流式端点：handler 返回 `StreamResponse<T>`，经 `CallStream` RPC 逐项回送 `CallResponse`——每项 success:true + JSON 数据；流式端点自带限流/认证/RBAC/载荷上限守卫链（`Call` 与 `CallStream` 两条 RPC 均为**限流最外层**——未认证洪水不进入恒时验证的 OS 线程睡眠，对齐 HTTP 栈「限流在认证外」的顺序）；项级错误以 success:false 的消息送达、流继续（对齐 SSE 错误事件语义）。unary 与流式注册表互斥：`Call` 打到流式方法、`CallStream` 打到 unary 方法均返回 `failed_precondition` 方向指引；流式路径不支持幂等重放（携带 `idempotency-key` 显式拒绝）。

两条流式生命周期边界：① `context` feature 的 request_id/trace_id 作用域覆盖 handler 主体（参数装配到返回 `StreamResponse`），不跨越后续流产出阶段（该阶段由 tonic 连接任务 poll，用户自行 spawn 的生产者任务也不继承 task_local）——逐项日志需要关联 id 时应在生产者任务中显式携带；② 生产者任务 panic/中止 → 发送端 drop → 流以**正常耗尽**收尾，客户端无法区分完整流与截断流——需要 fail-visible 的调用方应持有 JoinHandle 监测并在异常退出时注入项级错误（`Err(msg)` 项 → success:false、流继续）。客户端断开则约定为取消语义：发送端 `send` 返回 Err，生产者循环应退出。

```rust
use sdforge::streaming::create_stream_channel;
use sdforge::forge;

#[forge(
    name = "count_stream",
    version = "v1",
    grpc_method = "examples.count_stream",
    stream = true
)]
async fn count_stream(count: Option<u64>) -> Result<sdforge::streaming::StreamResponse<String>, sdforge::core::ApiError> {
    let (tx, response) = create_stream_channel::<String>(8);
    tokio::spawn(async move {
        for i in 0..count.unwrap_or(3) {
            if tx.send(Ok(format!("evt-{i}"))).await.is_err() {
                break; // 客户端断开 → 停止产出
            }
        }
    });
    Ok(response)
}
```

`grpc` 开而 `streaming` 关时，`stream = true` 的声明在编译期报错（fail-loud，方法不会无声消失）；`CallStream` 在 `streaming` 关闭的服务器上返回 `unimplemented`。

#### 自定义 tonic service 挂载（`extra_services`）

应用自有 proto service（如消费方自建 proto 生成的 `XxxServer<T>`）可与 `SdForgeService` 同端口共存，无需另起进程：`GrpcServerConfig.extra_services` 接受一组 `Arc<dyn Fn(&mut tonic::service::RoutesBuilder) + Send + Sync>` 回调，装配期在每个回调上 `add_service(...)` 注册任意实现了 `NamedService` 的 tonic service，然后统一挂上 server（`SdForgeService` 最后注册）：

```rust
use sdforge::grpc::{GrpcServerConfig, build_server_with_config};

let nexusflow_svc = /* NexusFlow 自建 proto 的 service 实现 */;
let mut config = GrpcServerConfig::default();
config.require_auth = false; // 或配置 auth（见下）
config.extra_services.push(std::sync::Arc::new(move |routes| {
    routes.add_service(nexusflow_v1::NexusFlowServer::new(nexusflow_svc.clone()));
}));
build_server_with_config("0.0.0.0:50051", config).await?;
```

语义边界：挂载的服务共享 server 级配置（连接上限、超时、keepalive、TLS）与 `security` feature 下的全局 JWT 认证拦截器（无凭证请求在触达自定义 service 之前即被拒绝）；`auth_verifier`（`GrpcAuthVerifier`）是 `SdForgeService` 的 per-call 校验，**不**作用于自定义 service——需要等效认证的应用应在自己的 service 内自行实现。路由形态为 `/{S::NAME}/*rest`（tonic axum 路由）：自定义 service 的 NAME 不得与 `sdforge.v1.SdForgeService` 冲突，冲突在装配期 panic（fail-loud），不做静默改名。克隆 `GrpcServerConfig` 共享同一回调列表（`Fn` 而非 `FnOnce`，闭包内以 `Arc` 捕获 service 并克隆构造）。

### HTTP TLS 终止

启用 `serve-tls` feature 后，`http::tls` 在进程内以 rustls（aws-lc-rs provider）终止 TLS：证书/密钥从 `TlsConfig` 指向的 PEM 文件加载（unix 下 group/other 可读的私钥文件会打 warn，建议 `chmod 600`），ALPN 可配置（缺省 `["h2", "http/1.1"]`），每个请求自动注入 `ConnectInfo<SocketAddr>`（TLS 直连无前置代理，限流/审计因此拿到不可伪造的客户端 IP）。停机编排复用 graceful 三阶段（停止接新 → 排空在途 → 停止钩子），`TlsServeConfig` 另提供三道预认证护栏——握手超时（默认 10s）、HTTP/1.1 头读取超时（默认 30s，仅作用 h1）与 HTTP/2 keep-alive 探测（默认 30s 间隔 / 20s 确认超时，握手后停滞不发帧的 h2 连接超窗断连，`with_http2_keepalive` 可调或关闭）：

```rust
use sdforge::config::TlsConfig;
use sdforge::http::tls::{TlsServeConfig, serve_with_graceful_shutdown_tls, tls_acceptor};
use sdforge::http::{default_shutdown_signal, GracefulShutdownConfig};

let tls = TlsConfig::new("cert.pem", "key.pem");
let acceptor = tls_acceptor(&tls)?;
let listener = tokio::net::TcpListener::bind("0.0.0.0:8443").await?;
let config = TlsServeConfig::default()
    .with_graceful(GracefulShutdownConfig::default());
serve_with_graceful_shutdown_tls(
    app,
    listener,
    acceptor,
    default_shutdown_signal(),
    config,
).await?;
```

证书轮换用 `ReloadingTls`：acceptor 与重载器编译期绑定（不存在「config 与 reloader 拆开导致热重载静默失效」的错误形态），换盘后调 `reload()`——注意它是阻塞操作（文件 IO + PEM 解析），定时器线程可直呼，tokio worker 上用 `reload_async()`。重载读到的密钥对与证书不匹配时保留旧证书并返回错误，不打断在线服务。`TlsConfig` 也可写在配置文件里（`server.tls`，`SdForgeConfig::validate()` 会校验路径非空与 ALPN 合法性）。gRPC 侧的对应物是 `GrpcServerConfig::tls`（`grpc-tls` feature，tonic `ServerTlsConfig` 接线）——两者文档互链、实现独立。

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
- **日志**：`logging` feature 提供 `StructuredLogger` / `init_global_logger`；`inklog` feature 将裸 `log` 调用桥接到 inklog `LoggerManager` 结构化管道（`init_inklog_logger()`），并启用 `#[forge::log]` 声明日志属性宏（进入/退出/耗时/错误 + DataMasker 脱敏；feature 关闭时使用该宏在编译期显性报错）
- **`#[forge::log]` 边界与语义**：`unsafe fn` 的 `unsafe` 限定原样传播到壳函数（调用方仍需 unsafe 块履行安全契约，宏不会把 unsafe fn 包装成安全函数）；壳函数不捕获 panic/任务取消——函数体 panic 或 async 任务被取消时不会产生 `fn_exit` 记录（需要失败可见性的调用方应在函数体内显式捕获，或在调用方监测 JoinHandle）；**不支持 trait impl 内的方法**（宏会在同一 impl 块内追加隐藏内部函数，trait impl 不允许额外成员）；`const`/变参在宏展开点 fail-loud 拒绝。载荷惰性渲染：目标日志级别被全局 max_level 过滤时，参数捕获与掩码渲染不执行（info 生产配置下 debug 进入日志零额外分配成本）；超过 64 KiB 的载荷在掩码前截断（inklog DataMasker 对 >1 MiB 输入会整体跳过掩码，壳层截断保证敏感值永不以明文入日志）；凭证键值对规则覆盖引号键形态（`"password":"hunter2"` 与 `password=hunter2` 同样被掩码）

### 客户端 SDK 生成

`sdk` feature 启用后，经保留 CLI 子命令从已注册路由生成客户端产物：

```bash
myapp sdk --lang all --output-dir ./sdks --reqwest
# → sdks/sdforge_client.rs   （零外部依赖 Rust client；--reqwest 追加 reqwest 传输）
# → sdks/sdforge_client.ts   （fetch + 内嵌类型定义）
```

Rust 产物以 `Transport` trait 抽象传输（实现一次即可接入任意 HTTP 栈），
返回原始 JSON 字符串，反序列化由使用方按自身模型做；`base_url` 由客户端
`call` 拼进完整 URL 后交给 `Transport::execute`（尾斜杠自动剥离，传输实现
收到的是完整 URL，不再需要自行拼接基址）；`--reqwest` 段经生成
文件内的 `#[cfg(feature = "reqwest")]` 门控，使用方需自行声明 `reqwest`
依赖。方法名在渲染前按 `(method, path)` 排序派生，产物与路由注册顺序无关
（确定性快照锁定）。库内亦可直接调用 `sdforge::sdk::generate_rust_client /
generate_typescript_client` 做定制化生成。

### 分布式限流与 Redis L2 缓存（多副本部署）

多副本部署时启用 `ratelimit-dist`（分布式限流）与 `cache-l2`（跨副本 L2 缓存）：

```rust,ignore
// 分布式限流：多副本共享 RedisDistributedLimiter 计数后端（Lua 原子窗口），
// 配额跨副本全局一致；单实例/测试用 InMemoryDistributedLimiter。
let backend = Arc::new(limiteron::limiters::RedisDistributedLimiter::new(cache, 100, 60_000));
let limiter = DistributedRateLimiter::new(backend, DistributedRateLimitConfig::new(100, Duration::from_secs(60)));

// L2 缓存：跨副本共享缓存层（L1 进程内缓存在前）
let l2 = RedisL2Cache::connect(&RedisL2CacheConfig::new(redis_url).with_default_ttl(Duration::from_secs(300))).await?;
```

**Redis 生产配置指引（AUTH + TLS）**：

- **认证（AUTH/ACL）**：连接串携带凭据 `redis://default:<AUTH>@host:6379/0`；
  凭据只经环境变量或 secret 管理注入，禁止写入代码/配置仓库。建议启用
  Redis 6+ ACL 限定应用账号权限（仅目标 db 的 get/set/del/eval）。
- **TLS**：跨网段/公网部署必须 `rediss://host:6379`（TLS 传输加密）；内网
  高信任环境可用 `redis://` 并以网络隔离（VPC/安全组）补偿，须在部署评审
  记录补偿措施。
- **键前缀**：多应用共享实例时必须配置互异的 `key_prefix`（限流
  `with_key_prefix` / L2 `with_key_prefix`），防键冲突与跨应用越权枚举。

**后端不可达行为（显式可配）**：

| 组件 | 策略 | 默认 | 理由 |
|------|------|------|------|
| 分布式限流 | fail-open（放行 + 60s 窗口限速告警）+ 适配层熔断（连续 5 次后端错误打开，打开期请求不经后端直接裁决，30s 后半开探测恢复；`with_circuit_failure_threshold` / `with_circuit_open_duration` 可调） | fail-open | 限流是保护性机制而非正确性机制：计数存储故障时 fail-close 会把存储故障放大为全服务不可用，违背可用性优先。熔断补足黑洞形态——后端不回错只熬满超时期间，请求不再每请求付满超时。需要硬安全语义（防爆破/防撞库）的部署显式切换 `BackendFailurePolicy::FailClose`（熔断打开期以 `CircuitBreakerError` 拒绝，不偷换成放行） |
| Redis L2 缓存 | 固有 fail-open（读故障 = miss 回源，写故障 = 跳过）；同步面（`SyncCache`）经 `block_in_place` 桥接仅多线程 runtime 可用，current-thread runtime 上恒 miss——异步中间件热路径必须用 `get_async`/`set_async`/`delete_async`/`contains_async`（无桥接） | fail-open（无开关） | 缓存丢失只影响命中率不影响正确性；缓存层不存在有意义的 fail-close 语义（拒绝服务不是缓存的职责）。装配期连接失败仍显性报错（启动即暴露配置错误） |

真实 Redis 集成测试门控：设置 `SDFORGE_TEST_REDIS_URL` 后运行
`cargo test --features "ratelimit-dist,cache-l2" --test dist_backends_tests`；
未设置时用例显式跳过（沙箱/CI 无 Redis 属环境限制）。

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

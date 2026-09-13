# 更新日志

本项目所有重要变更都会在此文件中记录。

格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
本项目遵循 [语义化版本规范](https://semver.org/lang/zh-CN/spec/v2.0.0.html)。

## 📋 目录

<details open>
<summary>📑 目录</summary>

- [Unreleased](#unreleased)
- [0.5.0-rc.3](#050-rc3---2026-09-10)
- [0.5.0-rc.2](#050-rc2---2026-09-07)
- [0.4.7](#047---2026-07-23)
- [0.4.6](#046---2026-07-22)
- [0.4.5](#045---2026-07-22)
- [0.4.4](#044---2026-07-18)
- [0.4.3](#043---2026-07-17)
- [0.4.2](#042---2026-07-15)
- [0.4.1](#041---2026-07-13)
- [0.4.0](#040---2026-07-13)
- [0.3.5](#035---2026-07-12)
- [0.3.4](#034---2026-07-12)
- [0.3.3](#033---2026-07-11)
- [0.3.0](#030---2026-07-04)
- [0.2.0](#020---2026-07-04)
- [0.1.0](#010---2026-01-19)

</details>

## [Unreleased]

### ⚠️ 破坏性变更 (Breaking Changes)

- **`security::api_key::AppApiKeyAuth::add_key_version` 签名变更**：返回类型从 `()` 改为
  `Result<(), String>`。当已存在的 key 元数据损坏/不可反序列化时，本方法现在返回 `Err`
  且**不注册任何凭据**（安全不变量：绝不产生"可认证但无法 revoke/rotate"的孤儿 key）。
  此前它会静默注册 key hash 后跳过元数据更新。调用方需追加 `?` / `.unwrap()`。
- **`error::api_error::ApiError` 新增 `QuotaExhausted { used, total }` 变体**：
  `RateLimitError::QuotaExhausted` 现在映射到该变体（HTTP 429），不再映射到
  `RateLimitExceeded` 并把 `used` 塞进 `window_seconds`（design.md D8 tech debt 已清偿）。
  对 `ApiError` 做穷尽 `match` 的下游代码需补充分支。
- **`config::server::ServerConfig` 新增 `max_body_size: usize` 字段**（默认 10 MiB，
  `#[serde(default)]` 兼容旧配置文件）。以结构体字面量构造 `ServerConfig` 的代码需补充
  该字段或使用 `..Default::default()`。
- **`security::bearer::BearerAuth`**：新增手动 `Drop`（销毁时 volatile 擦除 secret）与
  手动 `Debug`（secret 恒输出 `[REDACTED]`）。builder 现拒绝空 `audience`/`issuer`
  （返回 `AuthConfigError::InvalidSecret`）。
- **错误脱敏统一（行为变更）**：`ApiError::internal_*` 构造器现在强制对 message 执行
  敏感信息脱敏（JWT/密钥/信用卡/SSN/文件路径 + 500 字符截断）；`SdForgeError::Internal`
  的 `sanitized_message()` 与 `to_service_error()`、`ApiError::Internal` 的 `to_mcp_json()`
  现在统一输出通用文案 `"An internal error occurred. Please try again later."`，原始
  消息只保留在 `Display`/`Debug`（日志）中。断言原始消息会出现在外部输出的测试需更新。
- **基准目标更名**：Cargo.toml `[[bench]]` 目标 `axiom_bench` 更名为 `sdforge_bench`
  （原 `src/benches/axiom_bench.rs` 为空壳孤儿文件，已删除）。

### 新增 (Added)

- `http::VersionRedirectLayer` / `VersionRedirectService`：可注入 `VersionRouterConfig`
  的版本重定向层。此前 `version_redirect_middleware` 硬编码默认配置，导致
  `redirect_unknown` / `sunset_header` / `deprecated_versions` 三个配置字段全部无效。
- `core::RegexCache::common::is_strong_password()`：完整密码强度检查
  （≥8 位 + 小写/大写/数字/特殊字符各至少一个）。
- `ServerConfig::max_body_size`：请求体上限可配置化（此前硬编码 10 MB）。
- 回归测试：广播可达性、超大消息连接清理、深度嵌套空容器拒绝、孤儿 key 拒绝、
  rotate 后旧 key 失效、终态会话驱逐、审计并发不丢日志、超时丢弃计数等。

### 修复 (Fixed)

- **WebSocket**（`src/websocket/handler.rs`）：
  - 修复广播/推送整体失效：`handle_socket` 此前丢弃 `WebSocketConnection::new`
    返回的 receiver，manager 注册的所有连接均为死通道，`broadcast` 必然失败并误删
    连接。现在所有出站消息统一经通道由 forwarder 任务写回 socket。
  - 修复连接泄漏 DoS：超大消息早退路径跳过 `remove_connection` 且无 RAII 兜底，
    连接条目永久泄漏。现在以 Drop guard 保证所有退出路径清理。
  - 修复 JSON 深度检查绕过：`calculate_value_depth` 不计容器自身层级，
    17~128 层纯空容器嵌套（如 `[[[...]]]`）深度算成 0，绕过 `MAX_JSON_DEPTH=16`。
  - `MAX_STRING_LENGTH`（64KB）从 `cfg(test)` 文档性常量变为 `parse_websocket_message`
    强制校验（`id`/`method`/`error`/`event`）。
- **CORS**（`src/config/cors.rs`）：`build_cors_layer` 硬编码
  `.allow_methods(Any).allow_headers(Any)`，`allowed_methods` / `allowed_headers`
  配置完全无效。现在配置精确生效（空列表/`*` 保持 Any 兼容；非法头部名报错）。
- **API Key**（`src/security/api_key.rs`、`api_key_manager.rs`）：
  - 孤儿 key（见破坏性变更）；
  - `rotate_key` 无 `rotation_config` 时旧 key 永久有效（现在轮换即替换）；
  - 元数据反序列化 `i64 as u64` 回绕导致 `Instant` 运算 panic / 永不过期
    （现在钳制，fail-closed）；
  - `cleanup_versions` retain 路径不重算 `active_version_index` 导致活动版本丢失。
- **审计日志**（`src/security/audit/`）：
  - `log()` 对同用户日志列表的 get→push→set 无互斥，并发写互相覆盖丢审计记录
    （新增 `merge_lock`，log() 与 worker 合并共用）；
  - trait 路径 `total_log_count` 被新建 0 值计数器顶替，监控指标失真；
  - 信号量超时丢弃不递增 `dropped_log_count`；
  - builder 零值（`queue_size(0)` panic、`max_concurrent_ops(0)` 全超时、
    `max_logs_per_user(0)` 全丢弃）统一钳制为最小 1。
- **缓存**（`src/cache/cache_impl.rs`）：`SyncCache::delete` 在 backend 删除失败时
  仍返回 `existed=true`，违反 trait 契约；现在返回 `false`。
- **国际化**（`src/i18n/mod.rs`）：`translate_or_fallback` 两次独立加锁之间存在
  TOCTOU，`set_locale` 并发时用过期 locale 查表（现在单锁完成快照+查表）。
- **MRTR**（`src/mcp/mrtr.rs`）：`get_session` 静默吞掉毒化锁（现记录告警）；
  Completed/Cancelled 终态会话永不驱逐，积累至 `MAX_MRTR_SESSIONS` 后
  `create_session` 永久失败（现按超时窗口老化）。
- **HTTP 路由**（`src/http/version_routing.rs`）：重定向丢弃 query string
  （`/api/test?foo=bar` → `/api/v1/test`，现保留）；`sunset_header` 配置头名生效。
- **HTTP 中间件**（`src/http/http_impl.rs`）：`resolve_route_path` 的 `base_path[1..]`
  防御性切片（空串/多字节首字符 panic、无前导斜杠静默丢首字符）。
- **正则**（`src/core/regex_cache.rs`）：`password_strong` 注释宣称强制复杂度而实际
  仅查长度（现在文档诚实，完整检查请用 `is_strong_password`）。
- **错误**（`src/error/context.rs`）：`ErrorContext::current()` 的 `file`/`line`
  恒指向 context.rs 自身、`function` 恒为 `"()"`（现 `#[track_caller]` 捕获真实
  调用方，`function` 诚实为 `None`）。
- **JWT/Bearer**（`src/security/bearer/bearer_impl.rs`）：base64url 解码器查找表以
  0 初始化，任何非法字节被静默当作 `'A'` 解码（现以 0xFF 哨兵严格拒绝）。
- **基准正确性**（`src/benches/sdforge_bench.rs`）：cache_clear 首迭代后度量空缓存、
  失效基准把 O(n) 重填充计入度量（改 `iter_batched`）、eviction 吞吐声明 150 与
  实际 50 次操作不符、denied 路径 `let _ =` 掩盖回归（改断言）、
  `jwt_secret_validation` 重复度量生成成本（现仅度量校验）。

### 安全加固 (Security Hardening)

- CI workflows（ci.yml / codeql.yml / release.yml / tag-deleted.yml）的所有第三方
  action 引用从可变 tag 固定为 40 位 commit SHA（附版本注释）——消除供应链
  tag 劫持风险（tiangang SAST 扫描 Medium 发现）。
- `ApiError::internal_*` 构造器强制脱敏改为 feature 感知：`security` feature
  关闭时退化为原样存储，保证裸默认构建与 ratelimit-only 构建可编译。
- `to_service_error` 的 `Internal` 分支与 `sanitized_message` / `to_mcp_json`
  三轨完全统一：HTTP 500 响应体现在不可能携带原始内部消息。

### 已知依赖健康信号 (Known Dependency Signals)

- `bincode 2.0.1`：RUSTSEC-2025-0141 标记为 unmaintained（informational，非漏洞，
  trivy + cargo-audit 双通道均 0 CVE）。可留意 postcard/rkyv 等替代方案，无需
  紧急行动。

### 文档 (Documentation)

- `hash_key`：补充无盐 SHA256 存储的威胁模型说明（确定性查找前提 + 依赖 key 高熵，
  禁止低熵口令直入 `add_key`）。
- `key_id`：说明 64-bit 截断是有意的审计隐私取舍，不参与认证决策。
- `validate_key`：明确 `client_ip` 参数当前未使用（保持 API 兼容）。
- `build_with_redirect`：明确警示其不挂载安全中间件，生产用 `build_with_config`。
- `canonicalize_cache_key`：明确其为调用方工具函数，缓存内部不会自动调用。
- `VersionRouterConfig::supported_versions`：明确版本合法性门控委托给路由注册。
- `examples/src/security/api_key.rs`、`examples/src/websocket/chat.rs`：显著标注
  认证/WS 端点为演示桩，禁止复制到生产。

---

## [0.5.0-rc.3] - 2026-09-10

### Added

- **AppConfig 安全/缓存字段**：`AppConfig` 新增 `security: SecurityConfig` + `cache: CacheConfig`（feature-gated），builder 同步支持 `.security()` / `.cache()` 方法
- **`build_rate_limiter()` 自动装配**：`AppConfig::build_rate_limiter()` 从 `security.rate_limit` 配置自动构造 `LimiteronAdapter`
- **响应缓存中间件**：`ResponseCacheLayer` / `ResponseCacheMiddleware` — GET 路由自动缓存成功响应，key 经 `canonicalize_cache_key` 规范化，命中短路、未命中回源回写
- **`AuditSink` trait**：抽象审计日志存储后端（`write` / `read` / `clear`），内存环形缓冲保留为默认 sink
- **`InklogAuditSink`**：`inklog` feature 下桥接审计事件到 inklog 结构化输出管道

### Changed

- 移除 `ratelimit-http` feature 中的 `limiteron/tower-middleware` 死重使能（全仓 0 import，sdforge 自带 Tower middleware 实现）
- 版本递增至 `0.5.0-rc.3`
- trait-kit → `0.5.0-rc.3`、oxcache → `0.5.0-rc.4`、limiteron → `0.3.0-rc.3`、inklog → `0.3.0-rc.3`
- 新增 `[patch.crates-io]` 本地路径联调

---

## [0.5.0-rc.2] - 2026-09-07

### Added

- **custom success status code support for `#[forge]` macro** — `#[forge(status = <code>)]` 静态声明（如 `status = 201` 用于 POST 创建）+ `ServiceResponse::success_with_status(data, code)` 动态控制；零破坏现有 API（默认 200）；HTTP/gRPC 协议拉通；OpenAPI response code 同步
- **`#[forge]` 宏 `i18n_key` 参数 + `sdforge::i18n` 翻译注册表** — description 运行时翻译（MCP `build_tool_model` / CLI `build_subcommand` 已接入，OpenAPI/gRPC 计划中）；新增公共 API：`register_translation` / `set_locale` / `get_locale` / `translate_or_fallback` / `clear_translations`、`ApiMetadata::with_i18n_key()` / `i18n_key()`；`pub mod i18n` 由 `#[cfg(feature = "i18n")]` 调整为无条件编译
  - 迁移提示：`CliCommandRegistration` 新增 `pub i18n_key: Option<&'static str>` 字段——对结构体字面量构造方是字段新增（补 `i18n_key: None` 即可），经 `new()` / builder 路径的调用方无需改动

### Changed

- 依赖升级：rmcp 2.2→3.2（MRTR 模型：call_tool 返回 CallToolResponse 枚举、ListToolsResult 新增 result_type/ttl_ms/cache_scope）、simd-json 0.17→0.18、validator 0.20→0.21、icu 2.2→2.3.1、tokio 1.52→1.53；axum-test 保持 ^21（22 全 rc）
- 版本号递增至 `0.5.0-rc.2`（下一个 minor 预发布）

### 测试

- E2E 目录承载迁移：tests/e2e_advanced.rs 迁入 tests/e2e/ 并 `[[test]]` 注册（178 测试零损失）；docs/TEST_SCENARIOS.md 场景固化（12 域）；deny licenses clarify ×4

### 文档

- rmcp 版本描述 2.1→3.2（6 处）；安装示例统一 0.5.0-rc.2；CONTRIBUTING MSRV 对齐 1.97.1；sdforge-macros html_root_url 对齐

---

## [0.4.7] - 2026-07-23

### Changed

- 依赖版本约束移除波浪号（`~`）：`[workspace.dependencies]` 全部 41 项及 `examples/Cargo.toml` 6 项统一改为 Major.Minor 精确格式（如 `tokio ~1.52`→`1.52`、`serde ~1.0`→`1.0`、`tonic ~0.14`→`0.14`、`rmcp ~2.2`→`2.2` 等）
- 依赖刷新：移除波浪号上限后 `cargo update` 将 `tokio 1.52.4`→`1.53.1`、`uuid 1.23.5`→`1.24.0`（此前被 `~` 约束锁死在 1.52/1.23 小版本内）

### Security

- 补公开 `deny.toml` 中 `bincode` [RUSTSEC-2025-0141](https://rustsec.org/advisories/RUSTSEC-2025-0141.html)（unmaintained）的 ignore 决策（此前未在 CHANGELOG 披露）。bincode v2.0.1 为本 crate 直接依赖（`Cargo.toml`），其维护团队因 doxxing/harassment 事件永久停维，公告标注 "No safe upgrade available"。短期保留（功能稳定 + cargo-deny 持续监控公告），中期评估迁移至 `postcard` / `bitcode` / `rkyv`。本次仅透明化已知风险，无主动安全行为变更。
- `cargo deny check advisories bans` 通过，无已知漏洞

---

## [0.4.6] - 2026-07-22

### Fixed

- 修复 CI Clippy Lint job 失败：MSRV 从 1.89 升至 1.94（inklog 0.1.11+ 要求 rustc 1.94，`--all-features` CI 启用了可选的 inklog 依赖）
- 恢复 examples/Cargo.toml 的 `serde` 依赖：0.4.5 误删 serde 导致 `#[derive(Serialize, Deserialize)]` 编译失败（E0463: can't find crate for `serde`），影响 CI Build job 和 Release verify job
- 修复 `tests/e2e_advanced.rs` 中 17 个 clippy lint（MSRV 升级后新暴露）：移除 Copy 类型 `RegexCacheStats` 上的 `.clone()`、将 16 个常量断言转为编译时 `const { assert!(..) }` 块

---

## [0.4.5] - 2026-07-22

### 测试

- 新增 `tests/e2e_advanced.rs`（178 个测试）：覆盖 cache_advanced、config_advanced、error_advanced、ratelimit_error_advanced、logging_advanced、i18n_advanced、security_advanced、ratelimit_advanced、regex_cache_advanced、plugin_init_advanced、cross_module_advanced、validation_constants_advanced 共 12 个模块的未覆盖场景

### 维护

- 移除未使用依赖：serde（examples）

---

## [0.4.4] - 2026-07-18

### Fixed

- `extract_client_ip_core` 收紧：无 `ConnectInfo` 时不再 last-resort 信任 `X-Forwarded-For` / `X-Real-IP` 头，直接返回 `None`（调用方 fallback 至 `"unknown"`）。消除未配置 `ConnectInfo` 部署下 IP 限流/封禁被伪造头绕过的向量。两处生产调用点（`http_impl` 鉴权、`ratelimit` 适配器）已对 `None` 安全兜底，无 panic 风险。
- `#[forge]` 宏生成 `input_schema` 的 `required` 字段元素不再带多余引号。此前 `macros/src/lib.rs` 对字段名 `format!("\"{}\"", name)` 手动加引号，叠加 `serde_json::json!` 宏二次加引号，致 `required: ["\"message\""]`（元素内容带引号），`schema_validation` 永远匹配不上 args key，对 `#[forge]` 工具的 required / unknown-field 校验形同虚设。改为 `name.to_string()` 后校验生效（`test_vuln0002_valid_field_accepted` / `test_vuln0002_unknown_field_rejected` 转绿，cargo test --all-features 全量 0 failed）。

### Changed

- **BREAKING** `ApiError::Internal.context` 字段类型 `Option<ErrorContext>` → `Option<Box<ErrorContext>>`。消除 clippy 1.96 `result_large_err`（`ErrorContext` 含 `HashMap` 致 `ApiError` enum 变体超 128 字节阈值）。所有构造点（生产代码 `error/api_error.rs` + 集成测试）已同步 `Some(Box::new(ctx))`。
- clippy 1.96 兼容（语义不变）：`mcp/schema_validation.rs` 嵌套 `if let` → let-chain（edition 2024，消 `collapsible_if`）；`examples/tests/comprehensive_features.rs` `iter().any(|m| *m == x)` → `contains(&x)`（消 `manual_contains`）。
- `tests/integration/grpc_tests.rs` 3 处 `GrpcServerConfig` 构造补全 `rate_limiter` 字段（`None`）——该字段由 gRPC 限流加固引入，但测试构造此前被 clippy 门禁阻断从未编译，本次同步修复。

---

## [0.4.3] - 2026-07-17

### ⚠️ BREAKING CHANGES

统一 handler 契约 + 多协议运行时 dispatch：

- **`CliHandlerFn` 删除** — 不再有独立的 CLI handler 函数指针类型。CLI handler 现在通过 `CliHandlerRegistration` 复用统一的 `HandlerFn` 签名（与 HTTP / gRPC / MCP 一致）
- **handler 签名统一** — 所有协议的 handler 现在遵循 `fn(HandlerArgs, HandlerState) -> HandlerFuture` 契约。`HandlerArgs` 自动从 clap / tonic / axum extractor 构造；`HandlerState` 通过 `downcast_state::<T>()` 注入。返回类型约束为 `T: Serialize`
- **`GrpcServerConfig.state` 新增** — `GrpcServerConfig` 增加 `state: Option<Arc<dyn Any + Send + Sync>>` 字段，用于向 gRPC handler 注入应用状态（与 `CliBuilder::with_dependencies` 对齐）
- **`#[cfg(feature = "http")]` 门控 HTTP 代码生成** — `#[forge(path=..., method=...)]` 生成的 HTTP 路由注册代码现在被 `#[cfg(feature = "http")]` 包裹。下游 crate 启用 `mcp` 或 `grpc` 单独 feature 时不再因 `sdforge::http` / `sdforge::axum` 不存在而编译失败（feature 隔离）

### Added

- **`CliBuilder::execute()`** — 一站式 CLI 入口（async）：`build() → get_matches() → dispatch() → extract_value() → println! → std::process::exit(0/1)`。返回 `!`，调用方只需 `#[tokio::main] async fn main() { cli.execute().await }`
- **`sdforge::cli::dispatch`** — 暴露的自由 dispatch 函数（位于 `src/cli/dispatch.rs`，由 `cli::mod.rs` re-export），供不退出的自定义调用场景使用；`CliBuilder::execute()` 内部即调用此函数
- **`core::extract_value(&Value)`** — 智能返回值提取：`Value::String` → 原始串（无引号）；其他 → JSON 序列化
- **`core::downcast_state::<T>(HandlerState)`** — 运行时 state 类型转换，handler 中通过 `#[state] db: Arc<Db>` 参数声明，宏生成 `let db = downcast_state::<Db>(state)?;`
- **`pub use anyhow;` re-export**（gated by `mcp`）— `#[forge(tool_name = "...")]` 宏生成的 MCP tool impl 引用 `sdforge::anyhow::anyhow!` / `sdforge::anyhow::Error`，下游无需直接依赖 anyhow
- **`pub use tonic;` re-export**（gated by `grpc`）— 下游可使用 `sdforge::tonic::transport::Channel` 等
- **`pub use prost;` re-export**（gated by `grpc`）— 下游可派生 protobuf 消息
- **`pub use utoipa;` re-export**（gated by `openapi`）— 下游可使用 `#[utoipa::ToSchema]` 等 derive 宏
- **`prelude` 新增 re-export**：`pub use utoipa`（`openapi`）、`pub use anyhow`（`mcp`）— 让 `use sdforge::prelude::*;` 的下游 crate 中宏生成的 `#[utoipa::path]` 属性和 `anyhow::anyhow!` 调用都能解析，**无需下游 Cargo.toml 直接依赖任何框架库**
- **examples 综合测试** `examples/tests/comprehensive_features.rs` — 77 个测试覆盖全部 feature 的 re-export 可访问性、example 类型可构造性、inventory 注册计数、`#[forge]` 跨协议 dispatch（cli/grpc/mcp/openapi/http 从下游 crate 注册）

### Changed

- **logging feature 补 `dep:once_cell`** — `src/logging.rs` 使用 `once_cell::sync::OnceCell` 作为 `GLOBAL_LOGGER`，但 `logging` feature 未声明 `dep:once_cell`，导致 `--features logging` 单独编译失败。现已修正
- **macros 版本** 0.4.2 → 0.4.3

### Fixed

- 修复 `#[forge]` 宏生成代码中裸 `anyhow::anyhow!` / `anyhow::Error` 引用 → 改为 `sdforge::anyhow::anyhow!` / `sdforge::anyhow::Error`，下游无需直接依赖 anyhow
- 修复 `logging` feature 缺 `dep:once_cell` 导致单独编译失败（之前依赖 `http` feature 间接启用 `once_cell`）
- gRPC `call` 路径新增参数载荷大小上限（1 MiB，与 MCP `MAX_ARGUMENTS_SIZE_BYTES` 对齐），关闭此前绕过 MCP schema/大小校验的超大载荷 DoS 向量
- **[版本修正]** 发布版本号由误标的 0.5.0 修正为 0.4.3（Cargo.toml / macros / CHANGELOG 一致）

### ⚠️ Known Limitations（本次发布披露）

- `extract_client_ip_core` 在无 `ConnectInfo`（未配置 axum `with_make_service_with_connect_info`）的部署下，last-resort fallback 会直接信任 `X-Forwarded-For` / `X-Real-IP` 头。这是有意的文档化权衡（无 ConnectInfo 时无法获取真实 TCP 对端 IP），但意味着此类部署的 IP 限流/封禁可被伪造头绕过。生产部署**必须**配置 `ConnectInfo` 以启用不可伪造的 TCP 对端 IP 提取。后续版本计划将 fallback 改为仅在显式配置「无代理受信」时生效。

---

## [0.4.2] - 2026-07-15

### Added

- `cli::GlobalArg` — typed wrapper for clap global arguments with `long`, `default_value`, `help` builders
- `CliBuilder::with_global_arg()` — register global args on the top-level Command (inherited by subcommands)
- `mcp::serve_stdio()` — convenience wrapper around `rmcp::ServiceExt` + `rmcp::transport::stdio()`
- `pub use clap;` re-export (gated by `cli` feature) — downstream crates can use `sdforge::clap::Command` without a direct clap dep
- `pub use rmcp;` re-export (gated by `mcp` feature) — downstream crates can use `sdforge::rmcp` without a direct rmcp dep

### Changed

- regex `~1.12` → `~1.13`

---

## [0.4.1] - 2026-07-13

### ⚠️ BREAKING CHANGES（仅影响启用 `kit` feature 的用户）

- trait-kit 0.2 → 0.3（pre-1.0 minor bump，Cargo 视为不兼容）；启用 `kit` feature 的用户需同步升级

### Dependencies

- trait-kit 0.2 → 0.3（对齐 oxcache/dbnexus/inklog/limiteron 依赖链）
- sdforge-macros 0.4.0 → 0.4.1

### Changed

- 移除未使用导入：`src/security/audit/mod.rs`、`src/http/mod.rs`、`src/i18n/mod.rs`、`src/grpc/tests/grpc_service_tests.rs`
- `AuthGrpcInterceptor` 可见性从 `struct`（私有）扩展为 `pub(crate) struct`（测试可见性需求）
- 添加 `#[cfg(test)]` 条件编译标注以隔离测试专用 re-export（`sanitize_error_message`、`make_auth_interceptor`、`Registration`、`Ordering`）
- 同步更新 7 处源码文档注释中的 `trait-kit 0.2.2` → `trait-kit 0.3` 引用

---

## [0.4.0] - 2026-07-13

### ⚠️ BREAKING CHANGES

- `#[service_api]` 宏属性名重命名为 `#[forge]`（无向后兼容，用户明确要求单单词化）
- 所有使用 `#[service_api(...)]` 的代码需迁移为 `#[forge(...)]`
- 参数键名全部不变（name/version/path/method/cli/description/tool_name/cache_ttl/ws_path/grpc_method/no_prefix/streaming）
- 内部函数名 `parse_service_api_args` 保持不变（不影响用户 API）

### Changed

- `macros/src/lib.rs`: `pub fn service_api` → `pub fn forge`
- `src/lib.rs`: re-export `service_api` → `forge`（含 prelude）
- examples/tests/src 全量迁移 `#[service_api(...)]` → `#[forge(...)]`
- README.md / README_EN.md / Cargo.toml 注释同步更新

### Dependencies

- sdforge-macros 0.3.5 → 0.4.0

---

## [0.3.5] - 2026-07-12

### Changed

- 导入路径扁平化重构（commit 5deb561）：文件级导入提升到模块级，减少三级 crate 路径

### ⚠️ BREAKING CHANGES

- `error` 模块从 `src/core/error/` 迁移到 `src/error/`，导入路径 `crate::core::error::` → `crate::error::`
- 新增 `SdForgeResult<T>` 类型别名
- 跨 crate 引用更新：`limiteron::FlowGuardError` → `limiteron::LimiteronError`

### Dependencies

- trait-kit 0.2.3 → 0.2.5
- oxcache 0.3.6 → 0.3.7
- inklog 0.1.4 → 0.1.6
- limiteron 0.2.3 → 0.2.4

---

## [0.3.4] - 2026-07-12

### Changed
- MSRV 从 1.91 降回 1.85（与其他 base workspace crate 统一）
- inklog 依赖版本约束从 "0.1.4" 放宽到 "0.1"（x.x 格式）
- README 徽章合并为一行格式，移除不存在的 README_EN.md 链接
- ci.yml MSRV 环境变量更新为 1.85
- 移除 module.rs 中过时的 TypeId::of const fn 注释

---

## [0.3.3] - 2026-07-11

### 概览

无功能性代码变更，CI/clippy 修复和 MSRV 提升至 1.91

### 变更

- **edition 2024 升级** — 从 edition 2021 升级至 edition 2024，采用最新 Rust 语言特性
- **rust-version 1.85** — 最低支持的 Rust 版本提升至 1.85（edition 2024 所需）
- **MIT license 统一** — 所有源文件添加 `SPDX-License-Identifier: MIT` 头，许可证统一为 MIT
- **inklog 集成** — 新增 `inklog` feature，将裸 `log` 输出桥接到 inklog LoggerManager 结构化日志管道
- **i18n 国际化** — 新增 `i18n` feature，基于 ICU4X 2.x 提供本地化 HTTP 错误消息和格式化
- **文档标准化** — README.md 重构为标准格式，新增 CONTRIBUTING.md 和 AGENTS.md

---

## [0.3.0] - 2026-07-04

### 概览

本次发布聚焦于**安全加固**与**依赖精简**：移除 confers 集成与 CLI 工具以收敛职责边界，缓存底座切换至 oxcache 0.3.2，修复 diting 安全审计 10 项发现与 kueiku FMEA 分析 5 项 Bug。共计 2057 个测试全部通过。

#### BREAKING 变更 ⚠️

1. **移除 confers 集成** — 删除 6 个特性：`validation`、`schema`、`watch`、`audit`、`hot-reload`、`cli`。如需 confers 能力，用户在自身代码中集成。
2. **移除 CLI 工具** — 删除 `src/main.rs` 与 `src/cli/` 目录，`cli` 特性不再存在。
3. **缓存底座从 dashmap 切换至 oxcache 0.3.2** — `DashMapCache` 现为 `OxcacheSyncCache` 的类型别名（由 oxcache 的 `DashMapMemoryBackend` 支撑），内部所有 `DashMap` 用法替换为 `Mutex<HashMap>` 或 `RwLock<HashMap>`。
4. **移除 dashmap 依赖** — 不再出现在 Cargo.toml。
5. **ServerConfig 默认值变更** — `DEFAULT_HOST` 从 `"0.0.0.0"`（fail-open，绑定所有网卡）改为 `"127.0.0.1"`（fail-safe 回环）。`Default` 实现改用常量：host="127.0.0.1"、port=8080、request_timeout_secs=30。
6. **JWT 密钥强制最小 32 字符** — `MIN_SECRET_LENGTH=32` 常量现已实际用于校验，短于 32 字符的密钥将被拒绝并返回错误。
7. **CORS 校验收紧** — `"http://"`（仅 scheme 无 host）在 `validate()` 与 `build_cors_layer()` 中均被拒绝。
8. **AppConfigBuilder::build() 一致性修复** — 未设置 `timeout` 字段时默认填充 `Some(TimeoutConfig::default())`，与 `AppConfig::default()` 行为一致。

#### 安全修复（diting 审计 — 10 项）

- **HIGH-001**：缓存 `key_index`/`backend` 一致性竞态 — 现在在整个 backend 操作期间持有 index 锁
- **HIGH-002**：缓存静默吞错 — backend 错误现通过 `log::warn!` 记录，不再 `let _ =`
- **MED-001**：websocket 中 `RwLock` 中毒 — 所有 `.write().unwrap()`/`.read().unwrap()` 替换为感知中毒的 `match`/`if let Ok(...)` 模式
- **MED-002**：`regex_cache.rs` 与 `validation.rs` 中 `Mutex` 中毒 — 同样替换为感知中毒模式
- **MED-003**：`init_all_plugins` 中 `Mutex` 中毒 — `routes.lock().unwrap().len()` 替换为 `.lock().map(|g| g.len()).unwrap_or(0)`
- **MED-004**：CORS validate 不一致 — 在 scheme 校验后新增 host 校验
- **LOW-001**：`ServerConfig` fail-open 默认值 — 已修复（见 BREAKING 第 5 项）
- **LOW-002**：JWT 密钥无最小长度 — 已修复（见 BREAKING 第 6 项）
- **LOW-003**：WebSocket 认证文档告警 — 新增 Security Warning 文档注释
- **LOW-004**：`SecurityHeaders::relaxed()` CSP 告警 — 新增 Security Warning 文档注释

#### Bug 修复（kueiku FMEA 分析 — 5 项）

- **BUG-1 [严重]**：`remove_connection` usize 下溢 — 现先检查 `map.remove(id).is_some()` 再 `fetch_sub(1)`，防止 `usize::MAX` 下溢导致所有新连接被永久阻塞
- **BUG-2 [低]**：`check_and_record` 窗口重置 off-by-one — 窗口重置时计数设为 1（非 0），当前消息被计入（原先每窗口允许 max+1 条消息）
- **BUG-3 [中]**：`AppConfigBuilder::build()` timeout 不一致 — 已修复（见 BREAKING 第 8 项）
- **BUG-4 [中]**：缓存 backend 容量驱逐后 `key_index` 成为超集 — `find_keys_by_pattern` 现通过 `backend.exists()` 过滤并惰性清理过期索引项
- **BUG-5 [低]**：`get_stats` 静默丢弃浮点统计 — 现尝试 u64 → f64（rate/ratio/pct ×100）→ `log::warn!`（不再静默）

#### 依赖变更

- **新增**：`oxcache = { version = "0.3", features = ["memory"] }`（来自 crates.io）
- **移除**：`dashmap`、`confers`、`schemars`、`clap`、`tera`、`walkdir`
- **移除特性**：`validation`、`schema`、`watch`、`audit`、`hot-reload`、`cli`

#### 从 v0.2.0 迁移

1. 若使用 `cli` 特性或 `src/main.rs` 二进制，需在应用层自行实现 CLI
2. 若依赖 `confers` 特性（validation/schema/watch/audit/hot-reload），需在自身代码中直接集成 confers
3. `DashMapCache` 类型仍可编译（为 `OxcacheSyncCache` 别名），但底层实现已变更
4. `ServerConfig::default()` 现绑定 `127.0.0.1`，生产部署需显式配置 host
5. JWT 密钥若短于 32 字符将被拒绝，请更新密钥

#### 测试

- ✅ 2057 个测试全部通过（0 失败）
- ✅ clippy 零警告零错误

---

## [0.2.0] - 2026-07-04

### 概览

本次重大更新包含架构改进（统一注册系统、配置管理重构、安全模块增强、缓存系统优化）、MCP SDK 迁移、OpenAPI 自动生成、文件拆分与代码质量清理，以及性能基准文档。

#### BREAKING 变更 ⚠️

**MCP SDK 迁移（mcp-sdk 0.0.3 → rmcp 0.16）：**
- 移除 `mcp-sdk = "0.0"` 依赖，新增 `rmcp = { version = "0.16", features = ["server"] }`
- 移除 `initialize` 握手流程，改用 `server/discover` 端点（适配 MCP 2026-07-28 规范）
- 新增 `StatelessServerHandler` 无状态适配层，实现 `rmcp::ServerHandler` trait
- `RouteRegistration::register_mcp` 签名从 `fn register_mcp(&self, server: &mut Server)` 改为 `fn register_mcp(&self, registry: &mut dyn McpToolRegistry)`
- 新增 `Mcp-Method` 和 `Mcp-Name` HTTP 头解析（`parse_mcp_headers`）
- 新增 `cache_semantics` 模块处理 `ttlMs` 和 `cacheScope` 字段
- 新增 Multi Round-Trip Requests (MRTR) 支持，`MrtrSessionManager` 管理 300 秒超时会话

**配置验证统一：**
- `AuthConfig`、`ServerConfig`、`AppConfig` 的 `ValidateConfig` trait 实现委托给 inherent `validate()` 方法，消除双实现行为分叉

#### 新增特性

**统一注册系统：**
- 新增 `define_registration!` 宏，消除协议模块重复代码
- 新增 `Registration` trait，提供统一的协议注册接口
- HTTP、MCP、WebSocket、gRPC 四大协议模块全面采用统一注册系统
- 编译时协议选择，未使用的协议零编译代码

**OpenAPI 自动生成（新 `openapi` feature）：**
- 基于 utoipa 5.5.0 生成 OpenAPI 3.1 规范
- `#[service_api]` 宏在 `openapi` 特性启用时自动通过 `inventory::submit!` 注册 `OpenApiRouteInfo`
- 新增 `OpenApiBuilder` 链式构造器（`new().title().version().description().build()`）
- 新增 `generate_openapi_spec()` 函数收集所有注册路由生成完整规范
- 宏使用 `#[cfg(feature = "openapi")]` 门控，未启用时零运行时开销

**OpenAPI 路径参数自动映射：**
- `#[service_api]` 宏自动提取路径参数（如 `/users/:id`）并生成 OpenAPI 参数条目
- 新增 `rust_type_to_openapi_schema()` 将 Rust 基本类型映射到 OpenAPI (type, format) 对
- 新增 `OpenApiPathParam` 类型，通过 `OpenApiRouteInfo::with_path_params()` 注册
- 宏生成 `#[cfg_attr(feature = "openapi", utoipa::path(...))]` 属性，支持 utoipa 工具链发现

**配置管理：**
- 新增模块化配置文件：`app.rs`、`cache.rs`、`security.rs`
- 新增 Builder 模式支持，提供更友好的 API
- 新增集中式默认值管理
- 新增配置验证功能（需启用 `validation` feature）

**安全增强：**
- 新增 API Key 版本管理功能
- 新增 LRU 缓存管理器，防止内存增长
- 新增密钥轮换审计日志
- 新增密钥过期检查机制

**缓存优化：**
- 新增 `canonicalize_cache_key()` 键规范化函数
- 新增 `invalidate(pattern: &str)` 模式匹配失效
- 新增 `find_keys_by_pattern()` 正则表达式匹配
- 新增 `get_stats()` 统计信息跟踪
- 新增 `delete_many()` 批量删除操作

#### 文件拆分（降低单文件复杂度）

- `src/mcp/mod.rs` 拆分为 `server.rs`、`handler.rs`、`stateless.rs`、`headers.rs`、`cache_semantics.rs`、`mrtr.rs`、`protocol.rs` + `tests/`（mod.rs 从 800+ 行降至 200 行）
- `src/websocket/mod.rs` 拆分为 `connection.rs`、`handler.rs`、`broadcast.rs`、`message.rs` + `tests/`（mod.rs 从 2742 行降至 69 行）
- `src/core/error/mod.rs` 拆分为 `api_error.rs`、`i18n.rs`、`context.rs`、`sdforge_error.rs` + `tests/`（mod.rs 从 800+ 行降至 23 行）

- `src/security/audit.rs` (2210 行) → `audit/mod.rs` + `audit/tests/`（54 audit_logger + 10 builder 测试）
- `src/security/bearer.rs` (2107 行) → `bearer/mod.rs` + `bearer/tests/`（71 bearer_auth + 11 builder 测试）
- `src/security/types.rs` (1565 行) → `types/mod.rs` + `types/tests/`（75 types 测试）
- `src/http/mod.rs` (2210 → 434 行) + `tests/`（22 config + 44 routing + 3 middleware 测试）
- `src/grpc/mod.rs` (2067 → ~251 行) + `tests/`（88 grpc_service + 9 interceptor 测试）
- `src/streaming/mod.rs` (1458 → 231 行) + `tests/`（16 sse + 61 stream_builder 测试）

#### 改进

- 重构 HTTP 路由注册流程，减少样板代码
- 重构 MCP 工具注册流程，提高一致性
- 重构 WebSocket 路由注册，简化实现
- 重构 gRPC 路由注册，统一风格
- 优化缓存键处理，减少因格式不一致导致的 miss
- 优化错误处理，移除 CLI 相关变体（仅在 main.rs 中可用）
- audit 模块的 `eprintln!` 替换为 `log::warn!`（新增 `log` 工作区依赖）
- validation 模块移除 `#![allow(clippy::result_large_err)]`，改为 6 个函数级 `#[allow]`
- websocket/tests 和 core/error/tests 清理 `#![allow(unused_imports)]` 和未使用导入
- websocket handler/connection 添加 `#[cfg(feature = "security")]` 门控，使 `http,websocket`（无 security）编译通过
- 为 perf_* 示例添加 `required-features = ["cache"]` 声明
- 所有源文件添加 `SPDX-License-Identifier: MIT` 头

#### 正确性修复

- **CRIT-3**：移除 `macros/src/lib.rs` 中 `_param_unwraps` 的逐字重复定义
- **CRIT-4**：`AuthConfig`/`ServerConfig`/`AppConfig` 双 `validate()` 实现统一为单一来源
- **CRIT-5**：`MrtrSessionManager::create_session` 添加会话 ID 冲突检查，冲突时返回 `ErrorData::invalid_params`（原静默覆盖）
- **CRIT-6**：SSE 流 30 秒超时后发送 `Error` 事件，客户端可区分超时与正常完成
- **HIGH-003**：修复 `RegexCache` LRU 驱逐逻辑（`Reverse(time)` 导致驱逐 MRU 而非 LRU）
- **C-HIGH-1**：修复版本路由 `"v"` 单字符误判为有效版本（缺少 `len() > 1` 检查）
- **C-HIGH-4**：`ApiError::from_std_error` 中 `SystemTime::now().duration_since(UNIX_EPOCH).unwrap()` 改为 `.unwrap_or_default()`，防止系统时钟回拨 panic

#### 质量门禁

- 覆盖率从 88.96% 提升至 95.94%（2720/2835 行），超过 95% 目标
- `cargo clippy --all-features --all-targets -- -D warnings` 零警告零错误
- CI 覆盖率门禁修复：`--features full --lib` 替代 `--all-features --workspace` 避免 macros trybuild 测试超时
- `.tarpaulin.toml` 配置修正：`exclude_files` → `exclude-files`（kebab-case）
- diting 6 维度全量审查完成（Security/Performance/Quality/Architecture/Simplification/Correctness），6 项 P0 Critical/High 问题已修复
- 25 项 Medium/High 技术债记录至 `SIMPLIFY-DEBT.md` 作为 v0.2.1+ backlog

#### 文档

- `README.md` 改为中文版（原 `README_zh.md`），英文版迁移至 `README_EN.md`
- 移除所有 Redis 提及（与 no-db 策略一致）
- 修正 `CacheConfig` 文档示例字段（`ttl_seconds`/`max_size_mb`/`max_entries` → `default_ttl_secs`/`max_items`/`track_stats`）
- Feature 表格新增 `openapi`/`cli`/`validation`/`schema`/`watch`/`audit`/`simd-json`/`hex` 行，修正 `mcp`/`security`/`full` 依赖描述
- 新增 "OpenAPI 自动生成" 章节（README.md + README_EN.md）
- 新增 "MCP 2026-07-28 迁移指南" 章节（README.md + README_EN.md）
- 项目结构图补充 `streaming/` 模块
- 新增 `docs/benchmarks/vs-server-less.md` 性能基准文档（http vs full 编译时间/体积对比，实测数据）
- 新增 `SIMPLIFY-DEBT.md` 技术债清单（记录 25 项 Medium/High 技术债作为 v0.2.1+ backlog，含源代码路径索引）

#### 测试

- ✅ lib 测试 1638 个全部通过（基线 1383 + 新增 255）
- ✅ clippy 零警告零错误
- ✅ 覆盖率 95.94%
- ✅ CI 门禁本地验证通过

#### 技术债务

- 移除 `SdForgeError::Generator` 变体（CLI 模块不可用于 lib）
- 清理条件编译相关的 unreachable pattern
- `ApiError` 枚举体积过大（Internal/ServiceUnavailable 变体包含 `Box<dyn StdError>` + `ErrorContext`），导致 `clippy::result_large_err` 在 validation 模块 6 个函数上需局部 `#[allow]`；拆分 `ApiError` 记录为技术债

---

## [0.1.0] - 2026-01-19

### 初始发布

SDForge 框架首次发布（前身为 axiom，已于 2026-01-17 重命名为 sdforge）。

#### 核心特性

- **统一接口**：单个 `#[service_api]` 宏同时支持 HTTP 和 MCP
- **编译期协议选择**：通过 Cargo features 控制生成哪些协议
- **零运行时开销**：未使用的协议不会出现在二进制文件中
- **类型安全**：编译期验证 API 配置正确性

#### 支持的协议

- HTTP（通过 Axum 0.8.8）
- MCP（通过 mcp-sdk 0.0.3，后续在 0.2.0 迁移到 rmcp 0.16）

#### 支持的特性

| 特性 | 说明 | 默认启用 |
|------|------|---------|
| `http` | HTTP 服务器支持 | ✅ |
| `mcp` | MCP 协议支持 | ❌ |
| `streaming` | SSE 流式响应 | ❌ |
| `timestamp` | 响应时间戳 | ❌ |
| `logging` | 结构化请求日志 | ❌ |
| `security` | 安全认证和审计 | ❌ |
| `cache` | 响应缓存（LRU） | ❌ |
| `full` | 启用所有功能 | ❌ |

#### 安全功能

- Bearer Token 认证（HMAC-SHA256 JWT 验证）
- IP 白名单验证（拒绝私有/保留地址）
- 限流器（带幂等性支持）
- 审计日志（防 DoS 设计）
- 错误消息脱敏（防止信息泄露）

#### 缓存系统

- 基于内存的 HTTP 响应缓存
- ETag 和 Last-Modified 支持
- LRU 淘汰策略
- 可配置的大小和数量限制

#### 测试

- 单元测试（23+ 测试）
- 集成测试（HTTP、MCP、双协议）
- 缓存集成测试
- 配置集成测试
- 编译失败测试

#### 性能

- HTTP 请求处理：10,000+ req/s
- MCP 工具调用：5,000+ ops/s
- P50 延迟：< 0.5ms
- P95 延迟：< 1ms

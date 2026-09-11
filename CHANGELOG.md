# 更新日志 (Changelog)

本项目的所有显著变更将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

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
    17~128 层纯空容器嵌套（如 `[[[…]]]`）深度算成 0，绕过 `MAX_JSON_DEPTH=16`。
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

## [0.5.0-rc.3] - 2026-09-10

### 新增

- **限流配置化自动装配**：`SecurityConfig.rate_limit` 接入 `AppConfig` 与 `build_with_config`——配置存在即自动构造 `LimiteronAdapter` + `rate_limit_layer` 挂 HTTP 路由；`WebSocketConfig.rate_limit` 在 WS 握手层生效（T060）
- **`cache_ttl` 响应缓存闭环**：带 `cache_ttl` 的 GET 路由自动叠加 oxcache SyncCache 响应缓存层（key 使用 `canonicalize_cache_key`），命中短路、未命中回源后回写，MCP 成功路径同步回写；`config::CacheConfig` 接入 `AppConfig`（T061）
- **`AuditSink` 可插拔审计**：审计日志抽象 `AuditSink` trait（内存环形缓冲为默认 sink），`inklog` feature 下桥接 inklog 结构化输出（T062）

### 移除

- **`limiteron/tower-middleware` 死重使能**：全仓 0 import，限流统一走自研 `rate_limit_layer`（T060）

### 依赖

- inklog → `0.3.0-rc.3`、limiteron → `0.3.0-rc.3`、oxcache → `0.5.0-rc.4`、trait-kit → `0.5.0-rc.3`；开发期经 `[patch.crates-io]` 指向本地同级兄弟检出（发布 crates.io 后可移除 patch 段）

### 新增（workspace-rc4-completion Phase 7 追加，同版本节累计）

- **生产就绪**（T701/T702/T704）：
  - `health` feature——`build_with_config` 自动挂载 `/healthz`、`/readyz`（在认证层之后注册，天然 bypass 认证；`/readyz` 汇集 `ReadinessCheck` 与 kit 健康数据源，任一失败 503）；用户路由占用探针路径时自动让位（不 panic）
  - `metrics` feature——请求计数/延迟直方图/状态码分布自动采集（route 模板标签防基数爆炸），`/metrics` 端点 bypass 认证；自研轻量 Prometheus 文本渲染，零新增依赖
  - `graceful` feature——`serve_with_graceful_shutdown`：SIGTERM/SIGINT → 停止接新 → 排空在途（`drain_timeout` 上限强制退出）→ kit `shutdown_async` 三阶段关闭；集成测试覆盖在途完成、超时强停、kit 关闭钩子
- **声明式端点增强**（T703/T707/T708/T709/T711）：
  - `#[forge(auth(role = "admin"))]`——端点级 RBAC：路由包裹 `rbac::require_role`，`AuthContext` 无声明角色 → 403（默认拒绝）；`security` feature 关闭时 fail-safe 全拒绝
  - `#[forge(validate)]` + `#[param(ge/le/min_length/max_length/not_blank/email)]`——参数校验契约：400 响应携带字段级错误 `{"errors":[{field,rule,message}]}`
  - `#[forge(paginate)]`——声明式分页：自动 `page`/`size` 查询参数（默认 1/20，钳制 `1..=100`），`Vec<T>` 返回包装为 `{items,total,next}`
  - `etag` feature——GET 2xx 响应自动强 ETag（SHA-256），`If-None-Match` 命中/`*` → 304 空 body；POST 不受影响
  - `#[forge(on_start/on_stop)]`——进程生命周期钩子（inventory 注册，`lifecycle` feature），与 T704 停机顺序协同（on_start 先于监听、on_stop 在排空之后）
- **契约与观测**（T705/T706/T713/T714/T715）：
  - `context` feature——`RequestContext`（request_id/trace_id）经 tokio task_local 跨协议贯穿（HTTP 中间件/gRPC call/WS handle_socket/MCP dispatch），`StructuredLogger` 自动附加关联字段；响应回显 `X-Request-Id`/`X-Trace-Id`，支持 W3C `traceparent` 提取
  - OpenAPI Schema 补全——Body 参数生成 `requestBody`，返回类型映射生成响应 schema（`Result` 解包、`Vec<T>` → array、Rust 基元映射表）；`OpenApiRouteInfo` 新增 `body_params`/`response_type` 字段，运行时反射入口 `schema_for_type_name`
  - `hooks` feature——处理器前后钩子管道（`RequestHooks` before/after + 全局安装），hook panic 隔离不致请求失败；统一错误契约 `error::unified::UnifiedError`（code/message/trace_id/field），RBAC 403 与校验 400 均走同一渲染
  - `otel` feature——OTLP/HTTP JSON 导出（`/v1/traces` 请求 span + `/v1/metrics` 计数快照），零新增依赖（自研轻量 HTTP POST）；mock collector e2e
  - 宏诊断精确到参数 span——未知 `#[forge]` 键、`auth(...)` 内未知键、空 role 值均在出错 token 处报错（trybuild stderr 快照断言）
- **协议扩展**（T712）：
  - gRPC 认证拦截器——`SdForgeGrpcService::with_auth_interceptor`，凭据校验失败 → `Status::unauthenticated`；`GrpcAuthVerifier` 端口 + `BearerVerifier`（JWT）/`ApiKeyVerifier`（API key）适配器，复用 HTTP 凭据栈
  - WS 握手认证扩展——`WebSocketConfig.api_key_auth` 支持 `x-api-key` 凭据路径（与 bearer JWT 二选一通过即认证）
- **生态承接**（T716）：
  - 新顶层 crate `limiteron-admin`——limiteron 管理面多协议化（HTTP + CLI + MCP + OpenAPI 同一 `#[forge]` 声明集：status/check/introspect），含 6 项最小 e2e
  - oxcache 管理端点示例（经 `BackendRegistry` T315：kinds 列举 + 按名构建 + 读写往返）与 dbnexus 数据 API 网关对接示例（表/列白名单 + 过滤 + LIMIT/OFFSET 分页），各含最小 e2e
- **性能基线**（T710）：`benches/runtime_bench`（路由分发 plain/路径参数、HandlerArgs、JSON 序列化）+ `docs/PERFORMANCE.md` 本机基线（路由分发 ~553-604 ns，序列化 ~137 ns）

### 依赖（Phase 7 追加）

- 新增可选依赖：`futures-util`（lifecycle）、`sha2`（etag，复用既有 workspace 版本）；dbnexus `0.6.0-rc.3` 仅用于生态示例（examples 成员 path 依赖）；`trait-kit/health`、`trait-kit/lifecycle`、`oxcache/kit` 随对应 feature 级联启用
- 根包 `autoexamples = false`：`examples/` 目录归属 sdforge-examples 成员 crate，根包不再自动发现成员示例文件

## [0.5.0-rc.2] 及更早

此前版本无独立更新日志记录。

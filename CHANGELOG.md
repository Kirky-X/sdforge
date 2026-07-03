# 更新日志

本项目所有重要变更都会在此文件中记录。

格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
本项目遵循 [语义化版本规范](https://semver.org/lang/zh-CN/spec/v2.0.0.html)。

## [0.2.0] - 2026-07-04

### 概览

本次重大更新包含 Phase 1 架构改进（统一注册系统、配置管理重构、安全模块增强、缓存系统优化）、MCP SDK 迁移、OpenAPI 自动生成、文件拆分、代码质量清理（specmark code-quality-cleanup），以及性能基准文档。

#### BREAKING 变更 ⚠️

**MCP SDK 迁移（mcp-sdk 0.0.3 → rmcp 0.16）：**
- 移除 `mcp-sdk = "0.0.3"` 依赖，新增 `rmcp = { version = "~0.16", features = ["server"] }`
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

**Phase 1 拆分：**
- `src/mcp/mod.rs` 拆分为 `server.rs`、`handler.rs`、`stateless.rs`、`headers.rs`、`cache_semantics.rs`、`mrtr.rs`、`protocol.rs` + `tests/`（mod.rs 从 800+ 行降至 200 行）
- `src/websocket/mod.rs` 拆分为 `connection.rs`、`handler.rs`、`broadcast.rs`、`message.rs` + `tests/`（mod.rs 从 2742 行降至 69 行）
- `src/core/error/mod.rs` 拆分为 `api_error.rs`、`i18n.rs`、`context.rs`、`sdforge_error.rs` + `tests/`（mod.rs 从 800+ 行降至 23 行）

**代码质量清理拆分（specmark code-quality-cleanup）：**
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

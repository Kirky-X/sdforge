# SDForge 示例

本目录包含演示 SDForge 框架用法的全部示例，要求 Rust 1.97.1+（与工作区 MSRV 一致）。

更多示例运行方式与特性说明参见[主 README「💻 示例」章节](../README.md#-示例)。

## 目录

- [目录结构](#目录结构)
- [运行示例](#运行示例)
- [特性矩阵](#特性矩阵)
- [外部依赖](#外部依赖)
- [推荐学习路径](#推荐学习路径)
- [重点示例解析](#重点示例解析)
- [示例展示的最佳实践](#示例展示的最佳实践)
- [运行指定示例的测试](#运行指定示例的测试)

## 目录结构

示例分两类：**可运行示例**（顶层独立二进制，用 `cargo run --example` 执行）与**综合示例库**（`sdforge-examples` crate 的 lib 模块，位于 `src/`）。

### 可运行示例（顶层）

| 文件 | 所需特性 | 说明 |
|------|----------|------|
| `basic_cli.rs` | `cli` | `#[forge(cli = true)]` + `CliBuilder::execute()` 一站式 CLI（构建 / 解析 / 分发 / 输出 / 退出） |
| `swagger_demo.rs` | `docs` + `http` | 注册 HTTP 路由并挂载 Swagger UI（`/swagger-ui/`）与 OpenAPI JSON（`/api-docs/openapi.json`） |
| `perf_regex_cache.rs` | `cache` | 正则缓存性能验证 |
| `perf_lru_eviction.rs` | `cache` | LRU 驱逐性能验证 |
| `perf_prefix_index.rs` | `cache` | 前缀索引性能验证 |
| `perf_batch_ops.rs` | `cache` | 批量操作（`get_many` 等）性能验证 |

### 综合示例库（`src/`）

- `src/basics/` - 核心 API 定义与错误处理
  - `simple_api.rs` - 用 `#[forge]` 宏定义各类服务 API
  - `types_and_errors.rs` - 核心类型（如 `ApiError`）与错误处理模式
  - `response_building.rs` - 各类 HTTP 响应的构建方式
- `src/http/` - HTTP 协议示例（路由、参数、中间件）
  - `routing/path_params.rs` - 路径参数提取
  - `routing/query_params.rs` - 查询参数提取（可选性与默认值）
  - `middleware/cors.rs` - CORS 跨域资源共享配置
- `src/mcp/` - MCP 协议示例（工具定义与注册）
  - `tool_definition.rs` - 各类 MCP 工具的定义方式
  - `tool_registration.rs` - 基于 `inventory` 的工具自动注册
  - `migration_2026.rs` - MCP 2026-07-28 迁移（`mcp-sdk 0.0.3` → `rmcp 0.16`，`StatelessServerHandler` 无状态协议模型）
  - `mrtr_example.rs` - Multi Round-Trip Requests（MRTR）：通过 `InputRequiredResult` 挂起等待客户端补充输入，300 秒超时自动取消
- `src/security/` - 认证、授权、限流与审计日志
  - `api_key.rs` - API Key 认证的配置与使用
  - `auth_failures.rs` - 401 Unauthorized / 403 Forbidden 场景（中间件拦截未认证请求返回 401，权限不足由业务代码返回 403）
  - `comprehensive.rs` - 完整安全栈示例（API Key + JWT + 限流 + 审计 + 缓存）
- `src/cache/` - 内存缓存示例（oxcache/dashmap，无需外部服务）
  - `performance.rs` - 高级缓存模式（二级缓存、Cache-Aside、Write-Through）
- `src/config/` - 配置管理
  - `app_config.rs` - 应用配置的构建与加载
- `src/streaming/` - SSE 流式传输示例
  - `sse.rs` - Server-Sent Events 服务器推送
- `src/websocket/` - WebSocket 示例
  - `basic.rs` - 基础连接与消息处理
  - `chat.rs` - 实时聊天室
- `src/grpc/` - gRPC 示例
  - `server.rs` - gRPC 服务端：`GrpcRoute` 注册、`GrpcServerConfig`（连接数 / 超时 / 可选 JWT 认证）、`build_server` 启动
- `src/logging/` - 结构化日志示例
  - `structured.rs` - `StructuredLogger`、`LoggerConfig`、全局日志器与便捷宏
- `src/openapi/` - OpenAPI 3.1 规范生成示例
  - `basic.rs` - `generate_openapi_spec()` 自动收集路由、`OpenApiBuilder` 链式定制 `info` 段、手动注册 `OpenApiRouteInfo`
- `src/combined/` - 多特性组合的完整示例
  - `full_example.rs` - HTTP + MCP 等多特性集成
- `src/main.rs` - 综合演示服务器入口（见下文「启动演示服务器」）

其他：

- `config/` - 示例配置文件（`default.toml`、`minimal.toml`、`production.toml`、`api-key-auth.toml`）
- `tests/comprehensive_features.rs` - 通过 `sdforge-examples` crate 验证全部 sdforge 特性的集成测试

## 运行示例

### 启动演示服务器

默认 features 已启用全部 `*_examples`，在本目录（`examples/`）下直接执行：

```bash
cargo run                    # 启动演示服务器，监听 http://0.0.0.0:3000
HTTP_PORT=8080 cargo run     # 通过环境变量自定义端口
```

HTTP 端点：

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/hello` | GET | 问候语 |
| `/api/v1/users/:id` | GET | 获取用户 |
| `/api/v1/echo` | POST | 回显请求 |

WebSocket（`/ws/basic`、`/ws/chat`）与 SSE（`/stream/events`、`/stream/progress`）端点由对应示例模块注册。

### 运行单个可运行示例

以下命令在仓库根目录或 `examples/` 目录下均可执行：

```bash
# CLI 示例（echo 命令输出 Hello, world!）
cargo run --example basic_cli --features cli -- echo --name world

# Swagger UI 示例（启动后访问 http://127.0.0.1:8080/swagger-ui/，
# 端口可用 SDFORGE_HTTP_PORT 覆盖）
cargo run --example swagger_demo --features "docs http"

# 缓存性能验证（默认 features 已包含 cache，此处显式指定以便按需启用）
cargo run --example perf_regex_cache --features cache
cargo run --example perf_lru_eviction --features cache
cargo run --example perf_prefix_index --features cache
cargo run --example perf_batch_ops --features cache
```

注意：`src/` 下的综合示例库是 lib 模块，**不能**用 `cargo run --example` 运行，请参考文末「运行指定示例的测试」。

## 特性矩阵

`*_examples` 是面向用户的特性开关：每个开关同时启用对应的 sdforge 特性与本地别名特性（如 `cli`、`http`），使 `#[forge]` 宏生成的 `#[cfg(feature = "...")]` 门控在本 crate 内正确解析。

| `*_examples` 开关 | 启用的 sdforge 特性 | 覆盖示例 | 默认启用 |
|-------------------|---------------------|----------|----------|
| `http_examples` | `http` | `basics/`、`http/`、`config/` | ✓ |
| `mcp_examples` | `mcp` | `mcp/` | ✓ |
| `websocket_examples` | `websocket` | `websocket/` | ✓ |
| `streaming_examples` | `streaming` | `streaming/` | ✓ |
| `security_examples` | `security`（附带启用 `ratelimit`） | `security/` | ✓ |
| `cache_examples` | `cache`（隐含启用 `http_examples`） | `cache/` | ✓ |
| `grpc_examples` | `grpc` | `grpc/` | ✓ |
| `logging_examples` | `logging` | `logging/` | ✓ |
| `openapi_examples` | `openapi`（隐含启用 `http_examples`） | `openapi/` | ✓ |
| `cli_examples` | `cli` | `basic_cli.rs` | ✓ |
| `docs_examples` | `docs` | `swagger_demo.rs`（另需 `http`，运行命令见上文） | ✓ |
| `combined_examples` | 以上全部 | `combined/` | ✓ |

补充说明：

- 顶层 `perf_*.rs` 依赖 `cache` 特性（默认已启用）。
- `swagger_demo` 的 `[[example]]` 声明了 `required-features = ["docs", "http"]`。
- 精简构建示例：`cargo build --no-default-features --features "http_examples"` 只编译基础与 HTTP 相关示例。

## 外部依赖

所有示例均基于内存实现（DashMapCache/oxcache 等），**无需**数据库、Redis 等任何外部服务。

## 推荐学习路径

1. 从 `basics/simple_api.rs` 开始，理解 `#[forge]` 核心宏
2. 探索 `http/routing/path_params.rs` 与 `http/routing/query_params.rs`，掌握 HTTP 路由模式
3. 查看 `mcp/tool_definition.rs` 与 `mcp/tool_registration.rs`，了解 MCP 协议用法
4. 看 `combined/full_example.rs`，了解 HTTP + MCP 组合使用
5. 研究 `security/comprehensive.rs`，学习完整的安全实现
6. 阅读 `cache/performance.rs`，掌握高级缓存模式

## 重点示例解析

### 完整安全栈示例

对应文件：`security/comprehensive.rs`。演示一个生产可用的安全 API：

- **认证**：API Key（`X-API-Key` 请求头，适合服务间调用）+ JWT Bearer Token（用户认证）
- **授权**：基于角色的访问控制（Admin / User / Guest）
- **限流**：滑动窗口算法（100 次/分钟）
- **审计日志**：HMAC-SHA256 签名防篡改（设置 `SDFORGE_AUDIT_SIGNING_KEY` 环境变量后 `log()` 自动签名）
- **缓存**：带 TTL 的 LRU 缓存，自动失效
- **输入校验**：邮箱格式、密码强度（8–100 字符）、长度限制

关键代码（节选自源文件）：

```rust
use sdforge::security::{AppApiKeyAuth, AppAuditLogger, AuthContext, AuthMetadata, BearerAuth};

// API Key：注册密钥并绑定权限
let api_key_manager = AppApiKeyAuth::builder().build();
api_key_manager.add_key(
    "testkey_test_admin_123456".to_string(),
    vec!["admin".to_string(), "read".to_string(), "write".to_string()],
);

// JWT：生成安全随机密钥并构建 Bearer 认证器
let jwt_secret = sdforge::security::generate_secure_jwt_secret();
let _jwt_auth = BearerAuth::builder().secret(jwt_secret.clone()).build();

// 防篡改审计日志：设置 SDFORGE_AUDIT_SIGNING_KEY 后 log() 自动附加签名，
// 也可手动签名与校验（HMAC-SHA256）
state.audit_logger.log(&ctx, "app.start", "application", true, Some(detail)).await;
log.generate_signature(b"your-secret-key");
assert!(log.verify_signature(b"your-secret-key").unwrap());
```

演示端点：`GET /api/v1/users/:id`、`POST /api/v1/users`、`DELETE /api/v1/users/:id`（仅管理员）。

### 缓存性能优化示例

对应文件：`cache/performance.rs`。演示高级缓存策略：

- **二级缓存**：L1 存热数据（小而快）+ L2 存温数据（更大），L2 命中后自动提升回 L1
- **Cache-Aside 模式**：先查缓存，未命中再计算并回填（惰性加载，避免重复计算）
- **Write-Through 模式**：写缓存与更新存储同步进行，保证一致性
- **TTL 管理**：基于时间的过期
- **高效序列化**：`serde_json` 字节存储，`Cacheable` trait 约束可缓存类型

关键代码（节选自源文件）：

```rust
// 二级缓存：L1 最多 1000 条，L2 不限容量
let two_level = TwoLevelCache::new(1000);
pub struct TwoLevelCache {
    l1_cache: Arc<DashMapCache>, // 快速、小容量
    l2_cache: Arc<DashMapCache>, // 更大、较慢
    l1_max_size: usize,
}

// 读取顺序 L1 → L2，L2 命中后提升回 L1
if let Some(data) = self.l2_cache.get(key) {
    self.l1_cache.set(key, data.clone()); // 提升到 L1！
}

// Cache-Aside：先查缓存，未命中再计算并缓存结果
let cache_aside = CacheAsidePattern::new(Arc::new(DashMapCache::new()));
let result = cache_aside.get_or_compute(key, || async {
    expensive_computation().await
}).await;

// Write-Through：写入即更新缓存（真实应用中同步落库）
let write_through = WriteThroughPattern::new(Arc::new(DashMapCache::new()));
write_through.write(key, &value).await?;
```

## 示例展示的最佳实践

### 安全

✓ 永远不信任客户端输入——一切都要校验  
✓ 使用强密钥（JWT 建议 32 字符以上，可用 `generate_secure_jwt_secret()` 生成）  
✓ 为审计日志签名，防止篡改  
✓ 对所有端点限流，防止滥用  
✓ 缓存敏感数据时设置合适的 TTL  

### 性能

✓ 缓存昂贵的计算  
✓ 用二级缓存分离热数据与温数据  
✓ 写入时同步缓存（Write-Through）或显式失效  
✓ 高效地序列化 / 反序列化（字节格式）  
✓ 监控缓存命中率并调整 TTL  

### 代码质量

✓ 带上下文的完善错误处理  
✓ 类型安全的请求 / 响应模型  
✓ 多层输入校验  
✓ 清晰的职责分离  
✓ 充分的测试覆盖  

## 运行指定示例的测试

`src/` 下的示例是 `sdforge-examples` crate 的 lib 模块（不是 `cargo run --example` 二进制）。可以通过 `use sdforge_examples::security::comprehensive;` 这样的路径引用，也可以运行其模块测试（以下命令在仓库根目录执行）：

```bash
# 完整安全栈示例（运行模块测试）
cargo test --features "http_examples security_examples cache_examples" --manifest-path examples/Cargo.toml --lib security::comprehensive

# 缓存性能示例（运行模块测试）
cargo test --features "http_examples cache_examples" --manifest-path examples/Cargo.toml --lib cache::performance

# 全部特性组合（运行所有模块测试）
cargo test --manifest-path examples/Cargo.toml --lib
```

<div align="center">

<img src="docs/assets/sdforge.png" alt="SDForge Logo" width="180">

[![CI Status](https://github.com/Kirky-X/sdforge/actions/workflows/ci.yml/badge.svg)](https://github.com/Kirky-X/sdforge/actions/workflows/ci.yml) [![Version](https://img.shields.io/crates/v/sdforge.svg)](https://crates.io/crates/sdforge) [![Docs.rs](https://docs.rs/sdforge/badge.svg)](https://docs.rs/sdforge) [![Downloads](https://img.shields.io/crates/d/sdforge.svg)](https://crates.io/crates/sdforge) [![License](https://img.shields.io/crates/l/sdforge.svg)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.97.1%2B-orange.svg)](https://www.rust-lang.org/) [![Coverage](https://codecov.io/gh/Kirky-X/sdforge/branch/main/graph/badge.svg)](https://codecov.io/gh/Kirky-X/sdforge)

**中文** | [English](README_EN.md)

**一套宏注解，编译时装配多协议 SDK**

[✨ 功能特性](#-功能特性) • [🚀 快速开始](#-快速开始) • [📚 文档](#-文档) • [💻 示例](#-示例) • [🤝 参与贡献](#-参与贡献)

</div>

---

<div align="center" style="padding: 32px; margin: 24px 0">

### 🎯 一份注解，五种协议

用 `#[forge]` 宏标注一次函数，编译期生成 HTTP、MCP、gRPC、WebSocket、CLI 注册代码：

<table style="width:100%; border-collapse: collapse">
<tr><td align="center" width="25%" style="padding: 12px">🎯<br><b>统一注解</b><br><span style="color:#64748B">#[forge] 单宏定义端点，五协议消费同一份元数据</span></td><td align="center" width="25%" style="padding: 12px">⚡<br><b>编译时协议选择</b><br><span style="color:#64748B">feature 门控代码生成，未启用协议零编译代码</span></td><td align="center" width="25%" style="padding: 12px">🌐<br><b>五种协议入口</b><br><span style="color:#64748B">HTTP / MCP / gRPC / WebSocket / CLI</span></td><td align="center" width="25%" style="padding: 12px">🛡️<br><b>安全默认</b><br><span style="color:#64748B">认证、限流、审计，fail-safe 默认值</span></td></tr>
</table>

</div>

---

## 📋 目录

<details open>
<summary>📑 目录</summary>

- [✨ 功能特性](#-功能特性)
- [🚀 快速开始](#-快速开始)
- [🎨 特性标志](#-特性标志)
- [📚 文档](#-文档)
- [💻 示例](#-示例)
- [🏗️ 架构](#️-架构)
- [🧪 测试](#-测试)
- [📊 性能](#-性能)
- [🔒 安全](#-安全)
- [🗺️ 开发路线图](#️-开发路线图)
- [🤝 参与贡献](#-参与贡献)
- [📋 更新日志](#-更新日志)
- [📄 许可证](#-许可证)
- [🙏 致谢](#-致谢)
- [📞 联系与支持](#-联系与支持)
- [⭐ Star 历史](#-star-历史)

</details>

---

## ✨ 功能特性

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🎯 <b>统一接口定义</b><br><span style="color:#64748B">单个 <code>#[forge]</code> 宏同时配置 HTTP、MCP、gRPC、WebSocket、CLI</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">⚡ <b>零协议税</b><br><span style="color:#64748B">协议选择发生在编译期，运行期无协议探测或动态加载</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🌐 <b>多协议支持</b><br><span style="color:#64748B">Axum 0.8、rmcp 3.2（MCP 2026-07-28 规范）、tonic、WebSocket、SSE、clap</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🔒 <b>类型安全</b><br><span style="color:#64748B">接口定义编译期验证，trybuild 覆盖编译失败用例</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🛡️ <b>安全特性</b><br><span style="color:#64748B">API Key / JWT Bearer 认证、limiteron 限流、审计日志、安全头</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">💾 <b>内存缓存</b><br><span style="color:#64748B">oxcache 提供 LRU、模式失效、批量操作与统计，无数据库依赖</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔧 <b>配置管理</b><br><span style="color:#64748B">自包含 TOML 配置，模块化默认值与 Builder 模式</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">📊 <b>版本管理</b><br><span style="color:#64748B">内置 <code>/api/{version}</code> 多版本路由与 <code>#[service_module]</code> 模块前缀</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">📜 <b>OpenAPI 3.1</b><br><span style="color:#64748B">utoipa 编译期收集路由，运行时生成规范 + Swagger UI</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🌍 <b>国际化</b><br><span style="color:#64748B">ICU4X 本地化格式化与 Accept-Language 解析</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔭 <b>可观测性</b><br><span style="color:#64748B">Prometheus 指标、OTLP 导出、健康探针、优雅停机、请求上下文</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🧩 <b>特性组合</b><br><span style="color:#64748B">30+ Cargo features 按需装配，常驻核心不依赖任何协议栈</span></td>
</tr>
</table>

<details>
<summary>🔧 按需启用的进阶能力</summary>

参数校验、声明式分页、ETag 条件请求、生命周期钩子、钩子管道、优雅停机、健康探针、Prometheus 指标、OTel 导出、请求上下文、响应时间戳、结构化日志、inklog 桥接、trait-kit 集成、SIMD JSON 等能力均以独立 feature 按需启用，逐项说明与默认值见 [特性标志](#-特性标志) 一节。

</details>

---

## 🚀 快速开始

### 📦 安装

```bash
cargo add sdforge
```

或手动添加到 `Cargo.toml`（当前版本 `0.5.0-rc.3`）：

```toml
[dependencies]
sdforge = { version = "0.5.0-rc.3", features = ["http"] }
```

最低要求：

- **Rust 1.97.1+**（edition 2024，`rust-toolchain.toml` 固定工具链）
- **protoc**：仅在启用 `grpc` feature 时需要（`build.rs` 编译 protobuf）

> `sdforge` 默认不启用任何特性（`default = []`），按需显式启用协议特性。

### 💡 最小可运行示例

以下示例来自 [`examples/basic_cli.rs`](examples/basic_cli.rs)（`cli` feature）：

```rust
use sdforge::cli::CliBuilder;
use sdforge::core::ApiError;
use sdforge::forge;

#[forge(name = "echo", version = "1.0", description = "Echo a greeting", cli = true)]
async fn echo(name: String) -> Result<String, ApiError> {
    Ok(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() {
    sdforge::init_all_plugins();
    // execute() 返回 `!`：内部完成 build / parse / dispatch / 输出 / exit
    CliBuilder::new().execute().await;
}
```

```bash
cargo run --example basic_cli --features cli -- echo --name world
# 输出：Hello, world!（Value::String 智能提取，无引号）
```

### 🧭 核心概念

- `#[forge]` 注解描述一份端点元数据（名称、版本、路径、方法、描述），宏按启用的 feature 生成各协议注册代码，编译期经 `inventory::submit!()` 提交，启动时由 `init_all_plugins()` 一次性收集固化
- 按协议选择入口：`http::build()` / rmcp stdio / `SdForgeGrpcService` / `CliBuilder::execute()`

### 🔧 `#[forge]` 宏参数

`#[forge]` 必填 `name` 与 `version`；常用可选参数包括 `path` / `method` / `status` / `description` / `tool_name` / `grpc_method` / `cli`。完整参数表（含进阶参数与默认值）见 [API 参考](docs/API_REFERENCE.md#forge-参数) 的「`#[forge]` 参数」一节。

### 🌐 协议组合

| 目标 | features | 场景 |
|------|----------|------|
| 仅 HTTP | `["http"]` | 传统 REST API |
| 仅 MCP | `["mcp"]` | AI 工具集成 |
| HTTP + MCP 双协议 | `["http", "mcp"]` | 同一份代码双入口 |
| 全量运行时特性 | `["full"]` | 全部协议与能力（24 项；不含 `simd-json`/`kit`/`limiteron-integration` 可选重依赖） |

`grpc`、`websocket`、`streaming`、`openapi`、`cli`、`cache` 均可独立于 `http` 启用，任意组合。

---

## 🎨 特性标志

`default = []`：所有特性均为可选，按需显式启用。

<table>
  <tr><th>标志</th><th>说明</th><th>默认</th></tr>
  <tr><td><code>http</code></td><td>HTTP 服务器（Axum 0.8 路由、Tower 中间件、版本路由）</td><td>❌</td></tr>
  <tr><td><code>mcp</code></td><td>MCP 协议（rmcp 3.2，2026-07-28 规范：无状态 HTTP 头、MRTR、缓存语义）</td><td>❌</td></tr>
  <tr><td><code>grpc</code></td><td>gRPC（tonic + prost，独立于 http，proto 经 build.rs 生成）</td><td>❌</td></tr>
  <tr><td><code>websocket</code></td><td>WebSocket（依赖 http + streaming）</td><td>❌</td></tr>
  <tr><td><code>streaming</code></td><td>SSE 流式传输（独立于 http）</td><td>❌</td></tr>
  <tr><td><code>cli</code></td><td>CLI 集成（clap，独立于 http）</td><td>❌</td></tr>
  <tr><td><code>openapi</code></td><td>OpenAPI 3.1 规范生成（utoipa，独立于 http）</td><td>❌</td></tr>
  <tr><td><code>docs</code></td><td>统一文档输出（Swagger UI + CLI/MCP Markdown，依赖 openapi + cli）</td><td>❌</td></tr>
  <tr><td><code>security</code></td><td>认证（API Key / JWT Bearer）、审计、安全头、限流与缓存（含 http + ratelimit-http + cache）</td><td>❌</td></tr>
  <tr><td><code>ratelimit</code></td><td>限流核心（limiteron，不依赖 http）</td><td>❌</td></tr>
  <tr><td><code>ratelimit-http</code></td><td>HTTP 限流中间件（Tower Layer，依赖 http + ratelimit）</td><td>❌</td></tr>
  <tr><td><code>cache</code></td><td>oxcache 内存缓存（独立于 http）</td><td>❌</td></tr>
  <tr><td><code>health</code></td><td><code>/healthz</code> <code>/readyz</code> 健康探针（<code>build_with_config</code> 自动挂载，bypass 认证）</td><td>❌</td></tr>
  <tr><td><code>metrics</code></td><td>Prometheus 文本格式 <code>/metrics</code> 端点（请求计数、延迟直方图、状态码分布；自研轻量渲染）</td><td>❌</td></tr>
  <tr><td><code>graceful</code></td><td>优雅停机（SIGTERM/SIGINT：停止接新、排空在途、三阶段关闭）</td><td>❌</td></tr>
  <tr><td><code>context</code></td><td>请求上下文（request_id/trace_id 生成与跨协议 HTTP/MCP/gRPC/WS 注入）</td><td>❌</td></tr>
  <tr><td><code>validate</code></td><td><code>#[forge(validate)]</code> + <code>#[param(ge/le/...)]</code> 参数校验，400 返回字段级错误</td><td>❌</td></tr>
  <tr><td><code>paginate</code></td><td><code>#[forge(paginate)]</code> 声明式分页（自动 page/size 与 <code>{items,total,next}</code> 包装）</td><td>❌</td></tr>
  <tr><td><code>etag</code></td><td>ETag 条件请求（GET 响应自动附加 SHA-256 强 ETag，If-None-Match 返回 304）</td><td>❌</td></tr>
  <tr><td><code>lifecycle</code></td><td><code>#[forge(on_start/on_stop)]</code> 生命周期钩子（与优雅停机顺序协同）</td><td>❌</td></tr>
  <tr><td><code>hooks</code></td><td>处理器前后钩子管道（中间件式，统一错误契约）</td><td>❌</td></tr>
  <tr><td><code>otel</code></td><td>OTLP/HTTP JSON 导出请求 span 与指标快照（零额外依赖）</td><td>❌</td></tr>
  <tr><td><code>logging</code></td><td>结构化请求日志</td><td>❌</td></tr>
  <tr><td><code>timestamp</code></td><td>响应时间戳</td><td>❌</td></tr>
  <tr><td><code>inklog</code></td><td>inklog 结构化日志桥接</td><td>❌</td></tr>
  <tr><td><code>i18n</code></td><td>ICU4X 国际化（本地化格式化 + Accept-Language 解析）</td><td>❌</td></tr>
  <tr><td><code>simd-json</code></td><td>SIMD 加速 JSON 序列化/反序列化</td><td>❌</td></tr>
  <tr><td><code>limiteron-integration</code></td><td>引入 limiteron 依赖（kit 集成基座）</td><td>❌</td></tr>
  <tr><td><code>kit</code></td><td>trait-kit AsyncKit 集成（SdforgeModule 模块图）</td><td>❌</td></tr>
  <tr><td><code>tokio</code></td><td>内部特性：启用 tokio 依赖（随其他特性自动引入）</td><td>❌</td></tr>
  <tr><td><code>full</code></td><td>全部运行时特性（24 项：http/mcp/grpc/websocket/streaming/security/cache/health/metrics/graceful/context/validate/paginate/etag/hooks/lifecycle/otel/logging/timestamp/openapi/cli/docs/inklog/i18n；不含 <code>simd-json</code>/<code>kit</code>/<code>limiteron-integration</code> 可选重依赖）</td><td>❌</td></tr>
</table>

<details>
<summary>🔗 特性依赖关系</summary>

- 独立于 `http`：`mcp` / `grpc` / `openapi` / `cli` / `streaming` / `cache` / `timestamp` / `context` / `logging` / `inklog` / `i18n` / `simd-json` / `limiteron-integration`
- 派生自 `http`：`security`（含 `ratelimit-http` → `ratelimit` 与 `cache`）、`ratelimit-http`、`websocket`（含 `streaming`）、`health` / `metrics` / `graceful` / `validate` / `paginate` / `lifecycle` / `hooks` / `otel`
- `docs` = `openapi` + `cli`（Swagger UI 挂载需另启用 `http`）
- `kit` = `trait-kit`（health + lifecycle）+ `limiteron-integration` + `limiteron/kit` + `oxcache/kit`
- `full` 覆盖 24 项运行时特性，不含 `simd-json` / `kit` / `limiteron-integration`——三者是可选重依赖（SIMD JSON、trait-kit 模块图、限流集成基座），按需单独启用

</details>

---

## 📚 文档

| 文档 | 说明 |
|------|------|
| [📖 用户指南](docs/USER_GUIDE.md) | 从安装、核心概念到进阶用法的完整教程 |
| [📘 API 参考](docs/API_REFERENCE.md) | 核心类型与各 feature 门控的公开 API |
| [🏗️ 架构文档](docs/ARCHITECTURE.md) | 设计原则、模块划分、数据流、安全与性能设计 |
| [⚡ 性能基线](docs/PERFORMANCE.md) | 运行时热路径 criterion 基线与回归口径 |
| [📶 编译期门控基准](docs/benchmarks/vs-server-less.md) | feature 门控 vs 全量打包的编译时间与产物体积 |
| [🔒 安全文档](docs/SECURITY.md) | 漏洞报告流程、安全设计与最佳实践 |
| [🧾 测试场景](docs/TEST_SCENARIOS.md) | 测试金字塔基线与 E2E 场景定义 |
| [📋 更新日志](docs/CHANGELOG.md) | 每个版本的变更记录 |
| [🤝 贡献指南](docs/CONTRIBUTING.md) | 开发环境、TDD 工作流与 PR 流程 |
| [📦 在线 API 文档](https://docs.rs/sdforge) | docs.rs 自动生成的最新文档（all-features） |

---

## 💻 示例

### 根包示例（`cargo run --example`）

| 示例 | 所需特性 | 说明 |
|------|----------|------|
| `basic_cli` | `cli` | `#[forge(cli = true)]` + `CliBuilder::execute()` 一站式 CLI |
| `swagger_demo` | `docs` + `http` | Swagger UI 路由 + OpenAPI JSON + axum serve |
| `perf_regex_cache` | `cache` | 正则缓存性能验证 |
| `perf_lru_eviction` | `cache` | LRU 驱逐性能验证 |
| `perf_prefix_index` | `cache` | 前缀索引性能验证 |
| `perf_batch_ops` | `cache` | 批量操作性能验证 |

```bash
cargo run --example basic_cli --features cli -- echo --name world
cargo run --example swagger_demo --features "docs http"
cargo run --example perf_regex_cache --features cache
```

### 综合示例库（workspace 成员 `sdforge-examples`，`examples/src/`）

| 模块 | 内容 |
|------|------|
| `basics/` | 简单 API、响应构建、类型与错误处理 |
| `http/` | 路由（路径参数、查询参数）、中间件（CORS） |
| `mcp/` | 工具定义与注册、MCP 2026-07-28 迁移、MRTR 会话 |
| `security/` | API Key 认证、认证失败场景、完整安全栈 |
| `cache/` | 高级缓存模式与性能验证 |
| `config/` | 配置管理（`app_config.rs`） |
| `streaming/` | SSE 流式响应 |
| `websocket/` | 基础用法与聊天室 |
| `grpc/` | gRPC 服务端 |
| `logging/` | 结构化日志 |
| `openapi/` | OpenAPI 规范生成（`OpenApiBuilder`、`generate_openapi_spec`） |
| `combined/` | 多特性组合完整示例（`full_example.rs`） |

### 生态集成示例（`sdforge-examples` 成员示例）

| 示例 | 启用特性 | 说明 |
|------|----------|------|
| `oxcache_admin` | `oxcache_admin_example` | 经 `BackendRegistry` 暴露 oxcache 管理端点 |
| `dbnexus_gateway` | `dbnexus_gateway_example` | 白名单表上的只读数据 API 网关（sqlite 内存库） |

```bash
cargo run -p sdforge-examples --example oxcache_admin --features oxcache_admin_example
```

示例配置文件位于 `examples/config/`（`default.toml`、`minimal.toml`、`production.toml`、`api-key-auth.toml`）。

---

## 🏗️ 架构

SDForge 由两个 crate 组成：`macros/sdforge-macros` 负责解析 `#[forge]` / `#[service_module]` 注解并按 feature 门控生成注册代码；`sdforge` 是运行时库。注册项编译期经 `inventory::submit!()` 提交、启动期由 `init_all_plugins()` 固化，各协议模块（http / mcp / grpc / websocket / streaming / cli）彼此独立、按 feature 编译，共享 `core` 的统一 handler 契约（`HandlerArgs` + `HandlerState`）。模块注册全景、仓库布局与设计取舍见 [架构文档](docs/ARCHITECTURE.md)。

### 设计原则

编译时协议选择、inventory 注册模式、统一 handler 契约、三种构造模式（`new()` / `builder()` / `with_dependencies()`）、零数据库五大原则的完整阐述见 [架构文档](docs/ARCHITECTURE.md#-设计原则)。


### 🔄 核心执行路径

以 HTTP 请求热路径为例：请求经中间件栈（认证 → 限流 → 安全头 / CORS）进入版本路由匹配 `/api/{version}`，由 forge handler 在统一 handler 契约（`HandlerArgs` + `HandlerState`）中执行业务逻辑，经 `ServiceResponse` 响应管道输出 JSON；MCP、gRPC、CLI 入口复用同一批 handler 与响应契约，中间件栈顺序、版本路由与响应管道由 `http::build()` / `build_with_config()` 装配。完整时序图与数据流见 [架构文档](docs/ARCHITECTURE.md#-数据流)。


### 🌐 一份注解，五种协议

`#[forge]` 宏按当前启用的 feature 生成对应协议的注册项（HTTP / MCP / gRPC / CLI / OpenAPI），未启用的协议不生成任何代码；MCP 经 `Mcp-Method` / `Mcp-Name` 头（或 stdio）路由，gRPC 经 `SdForgeGrpcService::call()` 按 `grpc_method` 分发，CLI 经 `CliBuilder::execute()` 完成解析、分发与退出码，协议之间无运行时耦合。编译期注册流与请求期数据流详见 [架构文档](docs/ARCHITECTURE.md#-数据流)。

---

## 🧪 测试

### 测试策略矩阵

| 层级 | 位置 | 说明 |
|------|------|------|
| 单元测试 | `src/` 内嵌 `#[cfg(test)]`、`tests/unit/` | 模块级测试，含 proptest 属性测试（`src/tests/property_tests.rs`） |
| 集成测试 | `tests/integration/` | 覆盖 http / mcp / grpc / websocket / security / cache / streaming / openapi / cli / docs / health_probes / graceful_shutdown / validate / paginate / etag / lifecycle / otel_export / status_code / rbac 等协议与特性组合 |
| 宏测试 | `tests/macros/`、`macros/tests/` | trybuild 编译失败用例与宏展开验证 |
| E2E | `tests/e2e/` | `e2e_advanced` 多域 E2E 场景（12 域规模基线见 [测试场景](docs/TEST_SCENARIOS.md)） |
| 示例综合测试 | `examples/tests/` | 全 feature re-export 与跨协议 dispatch（77 个测试）及网关 E2E |
| 基准测试 | `benches/`、`src/benches/` | criterion：`runtime_bench` / `config_and_cache_bench` / `sdforge_bench` |
| Doc-tests | `src/` 文档注释 | rustdoc 内嵌示例 |

### 运行命令

```bash
# 与 CI 矩阵一致（http / mcp / http,mcp / http,security / http,cache /
# http,websocket / http,grpc / http,streaming / full 共 9 种组合）
cargo test --features "http,mcp" --workspace
cargo test --features full --workspace

# lib 测试（CI 覆盖率口径）
cargo test --features full --lib

# 覆盖率（CI 门禁 ≥80% 行覆盖，lefthook pre-push 同口径）
cargo llvm-cov --features full --lib --lcov --fail-under-lines 80

# 格式化与零告警 Lint
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

覆盖率测量经 `llvm-cov.toml` 排除 build.rs 生成的 protobuf 代码（`src/grpc/pb/`）。

### 测试规模

约 **2,900** 个测试函数（`src/` 2,015 + `tests/` 704 + `macros/` 59 + `examples/` 126，grep 统计，截至 v0.5.0-rc.3）。

---

## 📊 性能

### 运行时热路径（criterion 基线）

`plain_get` 路由分发中位延迟约 553 ns（约 1.81 M req/s），1 个路径参数约 604 ns；HandlerArgs 装配与 JSON 序列化/反序列化等完整基线、环境口径与复现命令见 [性能基线](docs/PERFORMANCE.md)。

### 编译时门控收益（feature 门控 vs 全量打包）

`http` only 相比 `full` 节省约 47% 编译时间、框架库 rlib 体积缩减约六成、唯一依赖少 82 个 crate；完整数据与方法论见 [编译期门控基准](docs/benchmarks/vs-server-less.md)（2026-07-03 实测，AMD Ryzen 9 9950X / rustc 1.93.1 / WSL2）。

---

## 🔒 安全

### 🚨 漏洞上报

**请勿通过公开 issue 报告安全漏洞。** 请使用 GitHub [Security Advisories](https://github.com/Kirky-X/sdforge/security/advisories/new) 私密披露通道提交，响应时限与流程见 [安全文档](docs/SECURITY.md)。

### 🛡️ 安全设计要点

认证（API Key / JWT Bearer 与密钥长度强制）、fail-safe 默认值（默认绑定 `127.0.0.1`）、不可伪造的客户端 IP、审计签名、错误脱敏、输入防御（MCP schema 校验与 1 MiB 载荷上限）等设计与取舍，详见 [架构文档](docs/ARCHITECTURE.md#-安全设计) 与 [安全文档](docs/SECURITY.md)。

### 🔍 供应链安全

CI 安全门禁常开：`cargo deny check`（[deny.toml](deny.toml) 策略）+ `cargo audit`，配合 CodeQL、Dependabot 与 pre-commit 密钥扫描（detect-secrets）；发布前另设 tiangang SAST 与 diting 审查门槛，见 [贡献指南](docs/CONTRIBUTING.md) 发布流程。

---

## 🗺️ 开发路线图

<table>
  <tr><th>状态</th><th>条目</th><th>说明</th></tr>
  <tr><td>🚧</td><td><b>v0.5.0 发布</b></td><td>当前处于 <code>0.5.0-rc.3</code>，推进依赖链（trait-kit / oxcache / inklog / limiteron）协同发布与终验</td></tr>
  <tr><td>✅</td><td>自定义成功状态码</td><td><code>#[forge(status = &lt;code&gt;)]</code> 静态声明 + <code>ServiceResponse::success_with_status</code> 动态控制，已于 0.5.0-rc.2 发布</td></tr>
  <tr><td>✅</td><td>MSRV 声明收敛</td><td>工作区统一为 1.97.1（2026-09-06），覆盖 <code>--all-features</code> 有效要求</td></tr>
  <tr><td>📋</td><td>错误码行为契约统一</td><td>评估同一校验错误在 HTTP（400）与 gRPC（422）间的状态码对齐（记录在案的行为契约）</td></tr>
  <tr><td>📋</td><td>依赖治理</td><td>中期评估将 <code>bincode</code>（RUSTSEC-2025-0141 unmaintained）迁移至 <code>postcard</code> / <code>bitcode</code> / <code>rkyv</code></td></tr>
</table>

---

## 🤝 参与贡献

欢迎贡献！开发环境初始化（工具链、protoc、lefthook / pre-commit 钩子）、TDD 工作流、特性组合校验与 Conventional Commits 提交规范（commit-msg 钩子强制校验），见 [贡献指南](docs/CONTRIBUTING.md)。

---

## 📋 更新日志

详见 [CHANGELOG.md](docs/CHANGELOG.md)。最近版本要点：

- **[0.5.0-rc.3]** (2026-09-10)：`ResponseCacheLayer` 响应缓存中间件、`AppConfig` security/cache 字段、`AuditSink` 审计存储抽象与 `InklogAuditSink`
- **[0.5.0-rc.2]** (2026-09-07)：`#[forge(status = <code>)]` 自定义成功状态码、`i18n_key` 参数与翻译注册表、rmcp 2.2 → 3.2
- **[0.4.7]** (2026-07-23)：依赖版本约束移除波浪号；补公开 `bincode` RUSTSEC-2025-0141 ignore 决策

---

## 📄 许可证

本项目基于 **MIT + Commons Clause** 双重条款发布：在 MIT 许可下可自由使用、修改与分发，但未经作者单独书面授权不得销售。详见 [LICENSE](LICENSE)。

Copyright (c) 2026 Kirky.X

---

## 🙏 致谢

SDForge 站在优秀的开源生态之上，感谢以下项目：

- [Axum](https://github.com/tokio-rs/axum) / [Tower](https://github.com/tower-rs/tower) — HTTP 服务与中间件
- [rmcp](https://crates.io/crates/rmcp) — MCP 官方 Rust SDK
- [Tonic](https://github.com/hyperium/tonic) / [Prost](https://github.com/tokio-rs/prost) — gRPC 与 protobuf
- [utoipa](https://crates.io/crates/utoipa) — OpenAPI 规范生成
- [clap](https://github.com/clap-rs/clap) — 命令行解析
- [inventory](https://crates.io/crates/inventory) — 编译期注册
- [ICU4X](https://github.com/unicode-org/icu4x) — 国际化
- base 工作区姊妹项目 [oxcache](https://github.com/Kirky-X/oxcache)、[limiteron](https://github.com/Kirky-X/limiteron)、[trait-kit](https://github.com/Kirky-X/trait-kit)、[inklog](https://github.com/Kirky-X/inklog)、[dbnexus](https://github.com/Kirky-X/dbnexus)

---

## 📞 联系与支持

- **🐛 Issue**：[github.com/Kirky-X/sdforge/issues](https://github.com/Kirky-X/sdforge/issues)
- **💬 讨论**：[github.com/Kirky-X/sdforge/discussions](https://github.com/Kirky-X/sdforge/discussions)
- **🏠 仓库**：<https://github.com/Kirky-X/sdforge>
- **📖 文档**：<https://docs.rs/sdforge>
- **👤 维护者**：Kirky.X

---

## ⭐ Star 历史

[![Star History Chart](https://api.star-history.com/svg?repos=Kirky-X/sdforge&type=Date)](https://star-history.com/#Kirky-X/sdforge&Date)

### 💝 支持本项目

如果您觉得这个项目有用，请考虑给它一个 ⭐️！

---

<div align="center">

**Built with ❤️ using Rust**

</div>

<div align="center">

<img src="docs/assets/sdforge.png" alt="SDForge Logo" width="240">

[![Rust 2024](https://img.shields.io/badge/Rust-2024-edded?logo=rust&style=for-the-badge)](https://www.rust-lang.org/) [![Rustc 1.97.1+](https://img.shields.io/badge/Rustc-1.97.1+-orange.svg?style=for-the-badge)](https://www.rust-lang.org/) [![crates.io](https://img.shields.io/crates/v/sdforge?style=for-the-badge)](https://crates.io/crates/sdforge) [![docs.rs](https://img.shields.io/docsrs/sdforge?style=for-the-badge)](https://docs.rs/sdforge) [![downloads](https://img.shields.io/crates/d/sdforge?style=for-the-badge)](https://crates.io/crates/sdforge) [![CI](https://img.shields.io/github/actions/workflow/status/Kirky-X/sdforge/ci.yml?branch=main&label=CI&style=for-the-badge)](https://github.com/Kirky-X/sdforge/actions/workflows/ci.yml) [![coverage](https://img.shields.io/codecov/c/github/Kirky-X/sdforge?style=for-the-badge)](https://codecov.io/gh/Kirky-X/sdforge) [![license](https://img.shields.io/crates/l/sdforge?style=for-the-badge)](LICENSE)

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
<td width="50%" style="vertical-align:top; padding: 12px">🌐 <b>多协议支持</b><br><span style="color:#64748B">Axum 0.8、rmcp 3.5（MCP 2026-07-28 规范）、tonic、WebSocket、SSE、clap</span></td>
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

参数校验、声明式分页、ETag 条件请求、生命周期钩子、钩子管道、优雅停机、HTTP TLS 终止、健康探针、Prometheus 指标、OTel 导出、请求上下文、响应时间戳、结构化日志、inklog 桥接、trait-kit 集成、SIMD JSON 等能力均以独立 feature 按需启用，逐项说明与默认值见 [特性标志](#-特性标志) 一节。

</details>

---

## 🚀 快速开始

### 📦 安装

<details>
<summary>📄 代码 · bash</summary>

```bash
cargo add sdforge
```

</details>

或手动添加到 `Cargo.toml`：

<details>
<summary>📄 代码 · toml</summary>

```toml
[dependencies]
sdforge = { version = "0.5.0-rc.9", features = ["http"] }
```

</details>

最低要求：

- **Rust 1.97.1+**（edition 2024，`rust-toolchain.toml` 固定工具链）
- **protoc**：仅在启用 `grpc` feature 时需要（`build.rs` 编译 protobuf）

> `sdforge` 默认不启用任何特性（`default = []`），按需显式启用协议特性。

### 💡 最小可运行示例

以下示例来自 [`examples/basic_cli.rs`](examples/basic_cli.rs)（`cli` feature）：

<details>
<summary>📄 代码 · rust</summary>

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

</details>

<details>
<summary>📄 代码 · bash</summary>

```bash
cargo run --example basic_cli --features cli -- echo --name world
# 输出：Hello, world!（Value::String 智能提取，无引号）
```

</details>

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
| 全量运行时特性 | `["full"]` | 全部协议与能力（26 项，含 `idempotency` 与部署面 HTTP TLS `serve-tls`/`tls-watch`；不含 gRPC 侧 `grpc-tls` 与 `simd-json`/`kit`/`limiteron-integration`/`schemars`/`ratelimit-dist`/`cache-l2`/`db-integration`/`sdk` 可选重依赖与 opt-in 面） |

`grpc`、`websocket`、`streaming`、`openapi`、`cli`、`cache` 均可独立于 `http` 启用，任意组合。

---

## 🎨 特性标志

`default = []`：所有特性均为可选，按需显式启用。

<details>
<summary>📋 表格</summary>

<table>
  <tr><th>标志</th><th>说明</th><th>默认</th></tr>
  <tr><td><code>http</code></td><td>HTTP 服务器（Axum 0.8 路由、Tower 中间件、版本路由）</td><td>❌</td></tr>
  <tr><td><code>mcp</code></td><td>MCP 协议（rmcp 3.5，2026-07-28 规范：无状态 HTTP 头、MRTR、缓存语义）</td><td>❌</td></tr>
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
  <tr><td><code>health</code></td><td><code>/healthz</code> <code>/readyz</code> 健康探针（<code>build_with_config_ext</code> 自动挂载，bypass 认证；inventory 注册的同名路由优先，<code>base</code> 声明的同名路径会在构建期 panic）</td><td>❌</td></tr>
  <tr><td><code>metrics</code></td><td>Prometheus 文本格式 <code>/metrics</code> 端点（请求计数、延迟直方图、状态码分布；自研轻量渲染；inventory 注册的同名路由优先，<code>base</code> 声明的同名路径会在构建期 panic）</td><td>❌</td></tr>
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
  <tr><td><code>kit</code></td><td>trait-kit AsyncKit 集成（SdforgeModule 模块图）</td><td>❌</td></tr>
  <tr><td><code>tokio</code></td><td>内部特性：启用 tokio 依赖（随其他特性自动引入）</td><td>❌</td></tr>
  <tr><td><code>full</code></td><td>全部运行时特性（26 项：http/mcp/grpc/websocket/streaming/security/cache/health/metrics/graceful/context/validate/paginate/etag/hooks/lifecycle/otel/timestamp/openapi/cli/docgen/inklog/i18n/idempotency/serve-tls/tls-watch；不含 gRPC 侧 <code>grpc-tls</code> 与 <code>simd-json</code>/<code>kit</code>/<code>limiteron-integration</code>/<code>schemars</code>/<code>ratelimit-dist</code>/<code>cache-l2</code>/<code>db-integration</code>/<code>sdk</code> 可选重依赖与 opt-in 面）</td><td>❌</td></tr>
</table>

</details>

<details>
<summary>🔗 特性依赖关系</summary>

- 独立于 `http`：`mcp` / `grpc` / `openapi` / `cli` / `streaming` / `cache` / `timestamp` / `context` / `inklog` / `i18n` / `simd-json` / `limiteron-integration` / `db-integration` / `schemars` / `ratelimit-dist`
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

---

## 📚 文档

<details>
<summary>📋 表格</summary>

| 文档 | 说明 |
|------|------|
| [📖 用户指南](docs/USER_GUIDE.md) | 从安装、核心概念到进阶用法的完整教程 |
| [📘 API 参考](docs/API_REFERENCE.md) | 核心类型与各 feature 门控的公开 API |
| [🏗️ 架构文档](docs/ARCHITECTURE.md) | 设计原则、模块划分、数据流、安全与性能设计 |
| [⚡ 性能基线](docs/PERFORMANCE.md) | 运行时热路径 criterion 基线与回归口径 |
| [📶 编译期门控基准](docs/benchmarks/VS_SERVER_LESS.md) | feature 门控 vs 全量打包的编译时间与产物体积 |
| [🔒 安全文档](docs/SECURITY.md) | 漏洞报告流程、安全设计与最佳实践 |
| [🧾 测试场景](docs/TEST_SCENARIOS.md) | 测试金字塔基线与 E2E 场景定义 |
| [📋 更新日志](docs/CHANGELOG.md) | 每个版本的变更记录 |
| [🤝 贡献指南](docs/CONTRIBUTING.md) | 开发环境、TDD 工作流与 PR 流程 |
| [📦 在线 API 文档](https://docs.rs/sdforge) | docs.rs 自动生成的最新文档（all-features） |

</details>

---

## 💻 示例

### 根包示例（`cargo run --example`）

<details>
<summary>📋 表格</summary>

| 示例 | 所需特性 | 说明 |
|------|----------|------|
| `basic_cli` | `cli` | `#[forge(cli = true)]` + `CliBuilder::execute()` 一站式 CLI |
| `swagger_demo` | `docgen` + `http` | Swagger UI 路由 + OpenAPI JSON + axum serve |
| `perf_regex_cache` | `cache` | 缓存键通配模式匹配验证（`find_keys_by_pattern`：`*` / `?` 正确性与单次全量扫描成本） |
| `perf_lru_eviction` | `cache` | 容量驱逐验证（`with_capacity` 的 FIFO 插入序驱逐：最旧键先出、读不续期 / 写续期） |
| `perf_prefix_index` | `cache` | 前缀索引性能验证 |
| `perf_batch_ops` | `cache` | 批量操作性能验证 |

</details>

<details>
<summary>📄 代码 · bash</summary>

```bash
cargo run --example basic_cli --features cli -- echo --name world
cargo run --example swagger_demo --features "docgen http"
cargo run --example perf_regex_cache --features cache
```

</details>

### 综合示例库（workspace 成员 `sdforge-examples`，`examples/src/`）

<details>
<summary>📋 表格</summary>

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
| `logging/` | 结构化日志（inklog LoggerManager：全局安装 / from_file 轮转 / builder / set_level） |
| `openapi/` | OpenAPI 规范生成（`OpenApiBuilder`、`generate_openapi_spec`） |
| `combined/` | 多特性组合完整示例（`full_example.rs`） |

</details>

### 生态集成示例（`sdforge-examples` 成员示例）

| 示例 | 启用特性 | 说明 |
|------|----------|------|
| `oxcache_admin` | `oxcache_admin_example` | 经 `BackendRegistry` 暴露 oxcache 管理端点 |
| `dbnexus_gateway` | `dbnexus_gateway_example` | 白名单表上的只读数据 API 网关（sqlite 内存库） |

<details>
<summary>📄 代码 · bash</summary>

```bash
cargo run -p sdforge-examples --example oxcache_admin --features oxcache_admin_example
```

</details>

示例配置文件位于 `examples/config/`（`default.toml`、`minimal.toml`、`production.toml`、`api-key-auth.toml`）。

---

## 🏗️ 架构

SDForge 由两个 crate 组成：`macros/sdforge-macros` 负责解析 `#[forge]` / `#[service_module]` 注解并按 feature 门控生成注册代码；`sdforge` 是运行时库。注册项编译期经 `inventory::submit!()` 提交、启动期由 `init_all_plugins()` 固化，各协议模块（http / mcp / grpc / websocket / streaming / cli）彼此独立、按 feature 编译，共享 `core` 的统一 handler 契约（`HandlerArgs` + `HandlerState`）。模块注册全景、仓库布局与设计取舍见 [架构文档](docs/ARCHITECTURE.md)。

### 设计原则

编译时协议选择、inventory 注册模式、统一 handler 契约、三种构造模式（`new()` / `builder()` / `with_dependencies()`）、零数据库五大原则的完整阐述见 [架构文档](docs/ARCHITECTURE.md#-设计原则)。


### 🔄 核心执行路径

以 HTTP 请求热路径为例：请求经中间件栈（限流 → 认证 → 安全头 / CORS）进入版本路由匹配 `/api/{version}`，由 forge handler 在统一 handler 契约（`HandlerArgs` + `HandlerState`）中执行业务逻辑，经 `ServiceResponse` 响应管道输出 JSON；MCP、gRPC、CLI 入口复用同一批 handler 与响应契约，中间件栈顺序、版本路由与响应管道由 `http::build()` / `build_with_config()` 装配。完整时序图与数据流见 [架构文档](docs/ARCHITECTURE.md#-数据流)。


### 🌐 一份注解，五种协议

`#[forge]` 宏按当前启用的 feature 生成对应协议的注册项（HTTP / MCP / gRPC / CLI / OpenAPI），未启用的协议不生成任何代码；MCP 经 `Mcp-Method` / `Mcp-Name` 头（或 stdio）路由，gRPC 经 `SdForgeGrpcService::call()` 按 `grpc_method` 分发，CLI 经 `CliBuilder::execute()` 完成解析、分发与退出码，协议之间无运行时耦合。编译期注册流与请求期数据流详见 [架构文档](docs/ARCHITECTURE.md#-数据流)。

---

## 🧪 测试

### 测试策略矩阵

<details>
<summary>📋 表格</summary>

| 层级 | 位置 | 说明 |
|------|------|------|
| 单元测试 | `src/` 内嵌 `#[cfg(test)]`、`tests/unit/` | 模块级测试，含 proptest 属性测试（`src/tests/property_tests.rs`） |
| 集成测试 | `tests/integration/` | 覆盖 http / mcp / grpc / websocket / security / cache / streaming / openapi / cli / docs / health_probes / graceful_shutdown / http_tls / validate / paginate / etag / lifecycle / otel_export / status_code / rbac 等协议与特性组合 |
| 宏测试 | `tests/macros/`、`macros/tests/` | trybuild 编译失败用例与宏展开验证 |
| E2E | `tests/e2e/` | `e2e_advanced` 多域 E2E 场景（11 域规模基线见 [测试场景](docs/TEST_SCENARIOS.md)） |
| 示例综合测试 | `examples/tests/` | 全 feature re-export 与跨协议 dispatch（76 个测试）及网关 E2E |
| 基准测试 | `benches/`、`src/benches/` | criterion：`runtime_bench` / `config_and_cache_bench` / `sdforge_bench` |
| Doc-tests | `src/` 文档注释 | rustdoc 内嵌示例 |

</details>

### 运行命令

<details>
<summary>📄 代码 · bash</summary>

```bash
# 与 CI 矩阵一致（http / mcp / http,mcp / http,security / http,cache /
# http,websocket / http,grpc / http,streaming / full 共 9 种组合）。
# 组合矩阵只测 sdforge 本体（-p sdforge）：--workspace 会因 sdforge-examples
# 的默认特性触发 feature 统一，把 sdforge 的几乎全部特性带进每个组合。
cargo test -p sdforge --features "http,mcp"

# 工作区全量（sdforge + sdforge-macros + sdforge-examples，含全部特性）
cargo test --workspace --all-features

# lib 测试（CI 覆盖率口径）
cargo test --features full --lib

# 覆盖率（CI 门禁 ≥90% 行覆盖：--features full --lib；lefthook pre-push 仅沿用同一阈值，未限定 --features full --lib）
cargo llvm-cov --features full --lib --lcov --fail-under-lines 90

# 格式化与零告警 Lint
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

</details>

build.rs 生成的 protobuf 代码写入 `OUT_DIR`，不再落源树；`llvm-cov.toml` 中的 `src/grpc/pb/*` 排除项为旧布局残留，当前不匹配任何文件。

### 测试规模

约 **3,181** 个测试函数（`src/` 2,180 + `tests/` 785 + `macros/` 92 + `examples/` 124，`grep -rEc '#\[(tokio::)?test\b' --include='*.rs'` 统计，2026-10-11 实测）。

---

## 📊 性能

### 运行时热路径（criterion 基线）

`plain_get` 路由分发中位延迟约 553 ns（约 1.81 M req/s），1 个路径参数约 604 ns；HandlerArgs 装配与 JSON 序列化/反序列化等完整基线、环境口径与复现命令见 [性能基线](docs/PERFORMANCE.md)。

### 编译时门控收益（feature 门控 vs 全量打包）

`http` only 相比 `full` 节省约 47% 编译时间、框架库 rlib 体积缩减约六成、唯一依赖少 82 个 crate；完整数据与方法论见 [编译期门控基准](docs/benchmarks/VS_SERVER_LESS.md)（2026-07-03 实测，AMD Ryzen 9 9950X / rustc 1.93.1 / WSL2）。

---

## 🔒 安全

### 🚨 漏洞上报

**请勿通过公开 issue 报告安全漏洞。** 请使用 GitHub [Security Advisories](https://github.com/Kirky-X/sdforge/security/policy) 私密披露通道提交，响应时限与流程见 [安全文档](docs/SECURITY.md)。

### 🛡️ 安全设计要点

认证（API Key / JWT Bearer 与密钥长度强制）、fail-safe 默认值（默认绑定 `127.0.0.1`）、不可伪造的客户端 IP、审计签名、错误脱敏、输入防御（MCP schema 校验与 1 MiB 载荷上限）等设计与取舍，详见 [架构文档](docs/ARCHITECTURE.md#-安全设计) 与 [安全文档](docs/SECURITY.md)。

### 🔍 供应链安全

CI 安全门禁常开：`cargo deny check`（[deny.toml](deny.toml) 策略）+ `cargo audit`，配合 CodeQL、Dependabot 与 pre-commit 密钥扫描（detect-secrets）；发布前另设 SAST 扫描与代码审查门槛，见 [贡献指南](docs/CONTRIBUTING.md) 发布流程。

---

## 🗺️ 开发路线图

<details>
<summary>📋 表格</summary>

<table>
  <tr><th>状态</th><th>条目</th><th>说明</th></tr>
  <tr><td>🚧</td><td><b>v0.5.0 发布</b></td><td>当前处于 <code>0.5.x</code> 发布线，推进依赖链（trait-kit / oxcache / inklog / limiteron）协同发布与终验</td></tr>
  <tr><td>✅</td><td>自定义成功状态码</td><td><code>#[forge(status = &lt;code&gt;)]</code> 静态声明 + <code>ServiceResponse::success_with_status</code> 动态控制，已于 0.5.0-rc.2 发布</td></tr>
  <tr><td>✅</td><td>MSRV 声明收敛</td><td>工作区统一为 1.97.1（2026-09-06），覆盖 <code>--all-features</code> 有效要求</td></tr>
  <tr><td>✅</td><td>错误码行为契约统一</td><td>行为契约记录在案（<code>docs/API_REFERENCE.md</code> 错误码行为契约表 + <code>error::unified</code> 单一事实来源）：HTTP 400（语法畸形/<code>InvalidInput</code>）与 422（语义违规/<code>ValidationError</code>）区分保留；gRPC 侧无 422 对应原生码，统一 <code>invalid_argument</code>、语义码经 <code>Status::details</code> 载荷 <code>code</code> 承载；<code>grpc_code_for_http_status</code> 契约表 + 跨协议一致性测试（全集 JOIN 不变量 + wire 级 e2e）钉死</td></tr>
  <tr><td>✅</td><td>依赖治理</td><td><code>bincode</code>（RUSTSEC-2025-0141 unmaintained）已整体迁移至 <code>postcard</code>（API Key 元数据/权限位序列化等价替换），<code>deny.toml</code> 失效豁免清除，<code>cargo deny</code> / <code>cargo audit</code> 复验为净</td></tr>
</table>

</details>

---

## 🤝 参与贡献

欢迎贡献！开发环境初始化（工具链、protoc、lefthook / pre-commit 钩子）、TDD 工作流、特性组合校验与 Conventional Commits 提交规范（commit-msg 钩子强制校验），见 [贡献指南](docs/CONTRIBUTING.md)。

---

## 📋 更新日志

详见 [CHANGELOG.md](docs/CHANGELOG.md)。最近版本要点：

- **[0.5.0-rc.8]** (2026-10-06)：MCP 认证贯通 call_tool 与 CLI 入口、`AuthConfig` 无 security 特性时 fail-closed；多协议契约对齐——Idempotency-Key 重放防护、RBAC/认证贯通 gRPC、统一错误映射、参数校验统一 422；异步健康检查与 `ReadinessRenderer` 渲染端口；OpenAPI 外部 spec 合并与 Swagger UI vendored 化（构建期零网络）；知识包 CLI（`--format json`/`docs --format agent`）；graceful 排空后钩子；三轮安全审查修复
- **[0.5.0-rc.5]** (2026-09-21)：维护性发布——`forge` status=204/304 无 body 响应、validator crate re-export、Swagger UI 自定义 spec 地址、i18n 整改、命名统一 `SdForge` 前缀、跨仓 path 依赖改走 crates.io 与工程加固
- **[0.5.0-rc.4]** (2026-09-14，含原 rc.3 批次)：`ResponseCacheLayer` 响应缓存中间件、`SdForgeConfig` security/cache 字段、`AuditSink` 审计存储抽象与 `InklogAuditSink`（0.5.0-rc.3 版本号跳过未发布，内容随本版发布）
- **[0.5.0-rc.2]** (2026-09-07)：`#[forge(status = <code>)]` 自定义成功状态码、`i18n_key` 参数与翻译注册表、rmcp 2.2 → 3.2
- **[0.4.7]** (2026-07-23)：依赖版本约束移除波浪号；补公开 `bincode` RUSTSEC-2025-0141 ignore 决策

---

## 📄 许可证

本项目基于 [MIT](LICENSE) 许可证发布。

Copyright (c) 2026 Kirky.X🌠

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

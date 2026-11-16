[English](README.md) | 中文

<div align="center">

<img src="docs/assets/sdforge.png" alt="SDForge Logo" width="240">

[![Rust 2024](https://img.shields.io/badge/Rust-2024-edded?logo=rust&style=for-the-badge)](https://www.rust-lang.org/) [![Rustc 1.97.1+](https://img.shields.io/badge/Rustc-1.97.1+-orange.svg?style=for-the-badge)](https://www.rust-lang.org/) [![crates.io](https://img.shields.io/crates/v/sdforge?style=for-the-badge)](https://crates.io/crates/sdforge) [![docs.rs](https://img.shields.io/docsrs/sdforge?style=for-the-badge)](https://docs.rs/sdforge) [![downloads](https://img.shields.io/crates/d/sdforge?style=for-the-badge)](https://crates.io/crates/sdforge) [![CI](https://img.shields.io/github/actions/workflow/status/Kirky-X/sdforge/ci.yml?branch=main&label=CI&style=for-the-badge)](https://github.com/Kirky-X/sdforge/actions/workflows/ci.yml) [![coverage](https://img.shields.io/codecov/c/github/Kirky-X/sdforge?style=for-the-badge)](https://codecov.io/gh/Kirky-X/sdforge) [![license](https://img.shields.io/crates/l/sdforge?style=for-the-badge)](LICENSE)

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
- [📚 文档](#-文档)
- [💻 示例](#-示例)
- [🏗️ 架构](#-架构)
- [🧪 测试](#-测试)
- [📊 性能](#-性能)
- [🔒 安全](#-安全)
- [🗺️ 开发路线图](#-开发路线图)
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

参数校验、声明式分页、ETag 条件请求、生命周期钩子、钩子管道、优雅停机、HTTP TLS 终止、健康探针、Prometheus 指标、OTel 导出、请求上下文、响应时间戳、结构化日志、inklog 桥接、trait-kit 集成、SIMD JSON 等能力均以独立 feature 按需启用，逐项说明与默认值见 [API 参考 · 特性标志](docs/cn/API_REFERENCE.md#-特性标志) 一节。

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
sdforge = { version = "0.5", features = ["http"] }
```

</details>

最低工具链要求（Rust 1.97.1+、`grpc` feature 所需 protoc）与「`default = []` 按需显式启用」说明见 [用户指南 · 快速开始](docs/cn/USER_GUIDE.md#-快速开始) 与 [API 参考 · 特性标志](docs/cn/API_REFERENCE.md#-特性标志)。

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

### 🔧 `#[forge]` 宏参数

`#[forge]` 必填 `name` 与 `version`；常用可选参数包括 `path` / `method` / `status` / `description` / `tool_name` / `grpc_method` / `cli`。完整参数表（含进阶参数与默认值）见 [API 参考](docs/cn/API_REFERENCE.md#forge-参数) 的「`#[forge]` 参数」一节。

### 🌐 协议组合

| 目标 | features | 场景 |
|------|----------|------|
| 仅 HTTP | `["http"]` | 传统 REST API |
| 仅 MCP | `["mcp"]` | AI 工具集成 |
| HTTP + MCP 双协议 | `["http", "mcp"]` | 同一份代码双入口 |
| MCP Streamable HTTP | `["mcp-http", "http"]` | `/mcp` 挂载（rmcp `StreamableHttpService`），零直连 rmcp |
| 全量运行时特性 | `["full"]` | 全部协议与能力（27 项，含 `idempotency` 与部署面 HTTP TLS `serve-tls`/`tls-watch`；不含 gRPC 侧 `grpc-tls` 与 `simd-json`/`kit`/`limiteron-integration`/`schemars`/`ratelimit-dist`/`cache-l2`/`db-integration`/`sdk` 可选重依赖与 opt-in 面） |

`grpc`、`streaming`、`openapi`、`cli`、`cache` 均可独立于 `http` 启用，任意组合；`websocket` 依赖 `http` + `streaming`。

---

## 📚 文档

<details>
<summary>📋 表格</summary>

| 文档 | 说明 |
|------|------|
| [📖 用户指南](docs/cn/USER_GUIDE.md) | 从安装、核心概念到进阶用法的完整教程 |
| [📘 API 参考](docs/cn/API_REFERENCE.md) | 核心类型与各 feature 门控的公开 API，含特性标志总表 |
| [🏗️ 架构文档](docs/cn/ARCHITECTURE.md) | 设计原则、模块划分、数据流、安全与性能设计 |
| [⚡ 性能基线](docs/cn/PERFORMANCE.md) | 运行时热路径 criterion 基线与回归口径 |
| [📶 编译期门控基准](docs/cn/VS_SERVER_LESS.md) | feature 门控 vs 全量打包的编译时间与产物体积 |
| [🔒 安全文档](docs/cn/SECURITY.md) | 漏洞报告流程、安全设计与最佳实践 |
| [🧾 测试场景](docs/cn/TEST_SCENARIOS.md) | 测试金字塔基线与 E2E 场景定义 |
| [📋 更新日志](docs/cn/CHANGELOG.md) | 每个版本的变更记录与开发路线图 |
| [🤝 贡献指南](docs/cn/CONTRIBUTING.md) | 开发环境、TDD 工作流与 PR 流程 |
| [📦 在线 API 文档](https://docs.rs/sdforge) | docs.rs 自动生成的最新文档（all-features） |

</details>

---

## 💻 示例

全部示例清单（根包示例、综合示例库 `sdforge-examples`、生态集成示例）与运行命令见 [用户指南 · 示例](docs/cn/USER_GUIDE.md#-示例)。

---

## 🏗️ 架构

模块注册全景、仓库布局与设计取舍见 [架构文档](docs/cn/ARCHITECTURE.md)，设计原则、数据流与安全/性能设计详见其中对应章节。

---

## 🧪 测试

测试策略矩阵、运行命令与规模基线见 [测试场景](docs/cn/TEST_SCENARIOS.md)。

---

## 📊 性能

### 运行时热路径（criterion 基线）

`plain_get` 路由分发中位延迟约 553 ns（约 1.81 M req/s），1 个路径参数约 604 ns；HandlerArgs 装配与 JSON 序列化/反序列化等完整基线、环境口径与复现命令见 [性能基线](docs/cn/PERFORMANCE.md)。

### 编译时门控收益（feature 门控 vs 全量打包）

`http` only 相比 `full` 节省约 47% 编译时间、框架库 rlib 体积缩减约六成、唯一依赖少 82 个 crate；完整数据与方法论见 [编译期门控基准](docs/cn/VS_SERVER_LESS.md)（2026-07-03 实测，AMD Ryzen 9 9950X / rustc 1.93.1 / WSL2）。

---

## 🔒 安全

### 🚨 漏洞上报

**请勿通过公开 issue 报告安全漏洞。** 请使用 GitHub [Security Advisories](https://github.com/Kirky-X/sdforge/security/policy) 私密披露通道提交，响应时限与流程见 [安全文档](docs/cn/SECURITY.md)。

### 🛡️ 安全设计要点

认证（API Key / JWT Bearer 与密钥长度强制）、fail-safe 默认值（默认绑定 `127.0.0.1`）、不可伪造的客户端 IP、审计签名、错误脱敏、输入防御（MCP schema 校验与 1 MiB 载荷上限）等设计与取舍，详见 [架构文档](docs/cn/ARCHITECTURE.md#-安全设计) 与 [安全文档](docs/cn/SECURITY.md)。

### 🔍 供应链安全

CI 安全门禁常开：`cargo deny check`（[deny.toml](deny.toml) 策略）+ `cargo audit`，配合 CodeQL、Dependabot 与 pre-commit 密钥扫描（detect-secrets）；发布前另设 SAST 扫描与代码审查门槛，见 [贡献指南](docs/cn/CONTRIBUTING.md) 发布流程。

---

## 🗺️ 开发路线图

当前路线图条目与状态见 [更新日志 · 开发路线图](docs/cn/CHANGELOG.md#开发路线图)。

---

## 🤝 参与贡献

欢迎贡献！开发环境初始化（工具链、protoc、lefthook / pre-commit 钩子）、TDD 工作流、特性组合校验与 Conventional Commits 提交规范（commit-msg 钩子强制校验），见 [贡献指南](docs/cn/CONTRIBUTING.md)。

---

## 📋 更新日志

详见 [CHANGELOG.md](docs/cn/CHANGELOG.md)。

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

English | [中文](README_CN.md)

<div align="center">

<img src="docs/assets/sdforge.png" alt="SDForge Logo" width="240">

[![Rust 2024](https://img.shields.io/badge/Rust-2024-edded?logo=rust&style=for-the-badge)](https://www.rust-lang.org/) [![Rustc 1.97.1+](https://img.shields.io/badge/Rustc-1.97.1+-orange.svg?style=for-the-badge)](https://www.rust-lang.org/) [![crates.io](https://img.shields.io/crates/v/sdforge?style=for-the-badge)](https://crates.io/crates/sdforge) [![docs.rs](https://img.shields.io/docsrs/sdforge?style=for-the-badge)](https://docs.rs/sdforge) [![downloads](https://img.shields.io/crates/d/sdforge?style=for-the-badge)](https://crates.io/crates/sdforge) [![CI](https://img.shields.io/github/actions/workflow/status/Kirky-X/sdforge/ci.yml?branch=main&label=CI&style=for-the-badge)](https://github.com/Kirky-X/sdforge/actions/workflows/ci.yml) [![coverage](https://img.shields.io/codecov/c/github/Kirky-X/sdforge?style=for-the-badge)](https://codecov.io/gh/Kirky-X/sdforge) [![license](https://img.shields.io/crates/l/sdforge?style=for-the-badge)](LICENSE)

**One macro annotation, multi-protocol SDKs assembled at compile time**

[✨ Features](#-features) • [🚀 Quick Start](#-quick-start) • [📚 Documentation](#-documentation) • [💻 Examples](#-examples) • [🤝 Contributing](#-contributing)

</div>

---

<div align="center" style="padding: 32px; margin: 24px 0">

### 🎯 One Annotation, Five Protocols

Annotate a function once with `#[forge]`; registration code for HTTP, MCP, gRPC, WebSocket, and CLI is generated at compile time:

<table style="width:100%; border-collapse: collapse">
<tr><td align="center" width="25%" style="padding: 12px">🎯<br><b>Unified Annotation</b><br><span style="color:#64748B">one #[forge] macro defines an endpoint, five protocols share metadata</span></td><td align="center" width="25%" style="padding: 12px">⚡<br><b>Compile-Time Selection</b><br><span style="color:#64748B">feature-gated generation, unused protocols compile to nothing</span></td><td align="center" width="25%" style="padding: 12px">🌐<br><b>Five Protocol Entrypoints</b><br><span style="color:#64748B">HTTP / MCP / gRPC / WebSocket / CLI</span></td><td align="center" width="25%" style="padding: 12px">🛡️<br><b>Secure Defaults</b><br><span style="color:#64748B">auth, rate limiting, audit with fail-safe defaults</span></td></tr>
</table>

</div>

---

## 📋 Table of Contents

<details open>
<summary>📑 Table of Contents</summary>

- [✨ Features](#-features)
- [🚀 Quick Start](#-quick-start)
- [📚 Documentation](#-documentation)
- [💻 Examples](#-examples)
- [🏗️ Architecture](#-architecture)
- [🧪 Testing](#-testing)
- [📊 Performance](#-performance)
- [🔒 Security](#-security)
- [🗺️ Roadmap](#-roadmap)
- [🤝 Contributing](#-contributing)
- [📋 Changelog](#-changelog)
- [📄 License](#-license)
- [🙏 Acknowledgments](#-acknowledgments)
- [📞 Contact & Support](#-contact--support)
- [⭐ Star History](#-star-history)

</details>

---

## ✨ Features

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🎯 <b>Unified interface definition</b><br><span style="color:#64748B">A single <code>#[forge]</code> macro configures HTTP, MCP, gRPC, WebSocket, and CLI at once</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">⚡ <b>Zero protocol tax</b><br><span style="color:#64748B">Protocol selection happens at compile time; no runtime probing or dynamic loading</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🌐 <b>Multi-protocol support</b><br><span style="color:#64748B">Axum 0.8, rmcp 3.5 (MCP 2026-07-28 spec), tonic, WebSocket, SSE, clap</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🔒 <b>Type safety</b><br><span style="color:#64748B">Interface definitions validated at compile time; trybuild covers compile-failure cases</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🛡️ <b>Security features</b><br><span style="color:#64748B">API Key / JWT Bearer auth, limiteron rate limiting, audit logging, security headers. Split as `security-core` (protocol-neutral, no HTTP) + `security-http` (HTTP adapters); `security` keeps the full aggregate</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">💾 <b>In-memory caching</b><br><span style="color:#64748B">oxcache provides LRU, pattern invalidation, batch ops, and stats; no database required</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔧 <b>Configuration management</b><br><span style="color:#64748B">Self-contained TOML config with modular defaults and Builder pattern</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">📊 <b>Versioning</b><br><span style="color:#64748B">Built-in <code>/api/{version}</code> version routing and <code>#[service_module]</code> module prefixes</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">📜 <b>OpenAPI 3.1</b><br><span style="color:#64748B">Routes collected at compile time via utoipa; runtime spec generation + Swagger UI</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🌍 <b>Internationalization</b><br><span style="color:#64748B">ICU4X locale-aware formatting and Accept-Language parsing</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔭 <b>Observability</b><br><span style="color:#64748B">Prometheus metrics, OTLP export, health probes, graceful shutdown, request context</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🧩 <b>Feature composition</b><br><span style="color:#64748B">30+ Cargo features assembled on demand; the always-on core depends on no protocol stack</span></td>
</tr>
</table>

<details>
<summary>🔧 Advanced capabilities, enabled on demand</summary>

Parameter validation, declarative pagination, ETag conditional requests, lifecycle hooks, hook pipeline, graceful shutdown, HTTP TLS termination, health probes, Prometheus metrics, OTel export, request context, response timestamps, structured logging, inklog bridge, trait-kit integration, and SIMD JSON are each enabled on demand via an individual feature; per-item descriptions and defaults are in the [API Reference · Feature Flags](docs/en/API_REFERENCE.md#-feature-flags) section.

</details>

---

## 🚀 Quick Start

### 📦 Installation

<details>
<summary>📄 Code · bash</summary>

```bash
cargo add sdforge
```

</details>

Or add it to your `Cargo.toml` manually:

<details>
<summary>📄 Code · toml</summary>

```toml
[dependencies]
sdforge = { version = "0.5", features = ["http"] }
```

</details>

For the minimum toolchain requirements (Rust 1.97.1+, protoc needed by the `grpc` feature) and the "`default = []`, enable features explicitly on demand" note, see the [User Guide · Quick Start](docs/en/USER_GUIDE.md#-quick-start) and [API Reference · Feature Flags](docs/en/API_REFERENCE.md#-feature-flags).

### 💡 Minimal runnable example

The following example comes from [`examples/basic_cli.rs`](examples/basic_cli.rs) (`cli` feature):

<details>
<summary>📄 Code · rust</summary>

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
    // execute() returns `!`: build / parse / dispatch / output / exit all happen inside
    CliBuilder::new().execute().await;
}
```

</details>

<details>
<summary>📄 Code · bash</summary>

```bash
cargo run --example basic_cli --features cli -- echo --name world
# Output: Hello, world! (smart Value::String extraction, no quotes)
```

</details>

### 🔧 `#[forge]` macro parameters

`#[forge]` requires `name` and `version`; common optional parameters include `path` / `method` / `status` / `description` / `tool_name` / `grpc_method` / `cli`. The full parameter table (including advanced parameters and defaults) is in the "`#[forge]` parameters" section of the [API Reference](docs/en/API_REFERENCE.md#forge-parameters).

### 🌐 Protocol combinations

| Goal | features | Scenario |
|------|----------|----------|
| HTTP only | `["http"]` | Traditional REST APIs |
| MCP only | `["mcp"]` | AI tool integration |
| HTTP + MCP dual protocol | `["http", "mcp"]` | One codebase, two entrypoints |
| MCP Streamable HTTP | `["mcp-http", "http"]` | Mount `/mcp` (rmcp `StreamableHttpService`) with zero direct rmcp dep |
| Full runtime features | `["full"]` | All protocols and capabilities (27 features, incl. `idempotency` and deployment-surface HTTP TLS `serve-tls`/`tls-watch`; excludes the gRPC-side `grpc-tls` and the optional heavy deps / opt-ins `simd-json`/`kit`/`limiteron-integration`/`schemars`/`ratelimit-dist`/`cache-l2`/`db-integration`/`sdk`) |

`grpc`, `streaming`, `openapi`, `cli`, and `cache` can all be enabled independently of `http`, in any combination (`websocket` requires `http` + `streaming`).

---

## 📚 Documentation

<details>
<summary>📋 Table</summary>

| Document | Description |
|----------|-------------|
| [📖 User Guide](docs/en/USER_GUIDE.md) | Complete tutorial from installation and core concepts to advanced usage |
| [📘 API Reference](docs/en/API_REFERENCE.md) | Public APIs of the core and every feature gate, incl. the feature flags table |
| [🏗️ Architecture](docs/en/ARCHITECTURE.md) | Design principles, module breakdown, data flow, security and performance design |
| [⚡ Performance Baselines](docs/en/PERFORMANCE.md) | Runtime hot-path criterion baselines and regression guidelines |
| [📶 Compile-Time Gating Benchmarks](docs/en/VS_SERVER_LESS.md) | Compile time and artifact size: feature gating vs full build |
| [🔒 Security](docs/en/SECURITY.md) | Vulnerability reporting process, security design, and best practices |
| [🧾 Test Scenarios](docs/en/TEST_SCENARIOS.md) | Test pyramid baseline and E2E scenario definitions |
| [📋 Changelog](docs/en/CHANGELOG.md) | Change log for every release, plus the roadmap |
| [🤝 Contributing](docs/en/CONTRIBUTING.md) | Development environment, TDD workflow, and PR process |
| [📦 Online API Docs](https://docs.rs/sdforge) | Latest docs.rs documentation (all features) |

</details>

---

## 💻 Examples

The full example inventory (root-package examples, the `sdforge-examples` comprehensive example library, ecosystem integration examples) and how to run them are in the [User Guide · Examples](docs/en/USER_GUIDE.md#-examples).

---

## 🏗️ Architecture

The full module-registration map and repository layout are in the [Architecture document](docs/en/ARCHITECTURE.md), with design principles, data flow, and security/performance design detailed in its corresponding sections.

---

## 🧪 Testing

The test strategy matrix, running commands, and scale baseline are in [Test Scenarios](docs/en/TEST_SCENARIOS.md).

---

## 📊 Performance

### Runtime hot path (criterion baselines)

`plain_get` routing dispatch runs at a median of ~553 ns (~1.81 M req/s), ~604 ns with 1 path parameter; the full baselines for HandlerArgs assembly and JSON serialization/deserialization, environment details, and reproduction commands are in [Performance Baselines](docs/en/PERFORMANCE.md).

### Compile-time gating gains (feature gating vs full build)

`http` only saves ~47% compile time, roughly 60% of framework rlib size, and 82 unique dependency crates compared to `full`; the full data and methodology are in the [compile-time gating benchmarks](docs/en/VS_SERVER_LESS.md) (measured 2026-07-03 on AMD Ryzen 9 9950X / rustc 1.93.1 / WSL2).

---

## 🔒 Security

### 🚨 Reporting vulnerabilities

**Do not report security vulnerabilities through public GitHub issues.** Please use the private GitHub [Security Advisories](https://github.com/Kirky-X/sdforge/security/policy) disclosure channel; response timelines and the process are in the [Security document](docs/en/SECURITY.md).

### 🛡️ Security design highlights

The design rationale for authentication (API Key / JWT Bearer with enforced key length), fail-safe defaults (binding `127.0.0.1` by default), unspoofable client IPs, audit signing, error sanitization, and input defense (MCP schema validation and 1 MiB payload caps) is in the [Architecture document](docs/en/ARCHITECTURE.md#-security-design) and the [Security document](docs/en/SECURITY.md).

### 🔍 Supply chain security

CI security gates are always on: `cargo deny check` ([deny.toml](deny.toml) policy) + `cargo audit`, together with CodeQL, Dependabot, and pre-commit secret scanning (detect-secrets); releases additionally require the SAST scan and code review gates — see the release process in the [Contributing Guide](docs/en/CONTRIBUTING.md).

---

## 🗺️ Roadmap

Current roadmap items and their status are in the [Changelog · Roadmap](docs/en/CHANGELOG.md#roadmap).

---

## 🤝 Contributing

Contributions are welcome! Development environment setup (toolchain, protoc, lefthook / pre-commit hooks), the TDD workflow, feature-combination checks, and the Conventional Commits message convention (enforced by the commit-msg hook) are described in the [Contributing Guide](docs/en/CONTRIBUTING.md).

---

## 📋 Changelog

See [CHANGELOG.md](docs/en/CHANGELOG.md).

---

## 📄 License

This project is licensed under the [MIT](LICENSE) License.

Copyright (c) 2026 Kirky.X🌠

---

## 🙏 Acknowledgments

SDForge stands on the shoulders of an excellent open-source ecosystem. Thanks to:

- [Axum](https://github.com/tokio-rs/axum) / [Tower](https://github.com/tower-rs/tower) — HTTP service and middleware
- [rmcp](https://crates.io/crates/rmcp) — the official MCP Rust SDK
- [Tonic](https://github.com/hyperium/tonic) / [Prost](https://github.com/tokio-rs/prost) — gRPC and protobuf
- [utoipa](https://crates.io/crates/utoipa) — OpenAPI spec generation
- [clap](https://github.com/clap-rs/clap) — command-line parsing
- [inventory](https://crates.io/crates/inventory) — compile-time registration
- [ICU4X](https://github.com/unicode-org/icu4x) — internationalization
- Base workspace sibling projects [oxcache](https://github.com/Kirky-X/oxcache), [limiteron](https://github.com/Kirky-X/limiteron), [trait-kit](https://github.com/Kirky-X/trait-kit), [inklog](https://github.com/Kirky-X/inklog), [dbnexus](https://github.com/Kirky-X/dbnexus)

---

## 📞 Contact & Support

- **🐛 Issues**: [github.com/Kirky-X/sdforge/issues](https://github.com/Kirky-X/sdforge/issues)
- **💬 Discussions**: [github.com/Kirky-X/sdforge/discussions](https://github.com/Kirky-X/sdforge/discussions)
- **🏠 Repository**: <https://github.com/Kirky-X/sdforge>
- **📖 Documentation**: <https://docs.rs/sdforge>
- **👤 Maintainer**: Kirky.X

---

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Kirky-X/sdforge&type=Date)](https://star-history.com/#Kirky-X/sdforge&Date)

### 💝 Support This Project

If you find this project useful, please consider giving it a ⭐️!

---

<div align="center">

**Built with ❤️ using Rust**

</div>

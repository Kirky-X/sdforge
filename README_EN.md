<div align="center">

<img src="docs/assets/sdforge.png" alt="SDForge Logo" width="240">

[![Rust 2024](https://img.shields.io/badge/Rust-2024-edded?logo=rust&style=for-the-badge)](https://www.rust-lang.org/) [![Rustc 1.97.1+](https://img.shields.io/badge/Rustc-1.97.1+-orange.svg?style=for-the-badge)](https://www.rust-lang.org/) [![crates.io](https://img.shields.io/crates/v/sdforge?style=for-the-badge)](https://crates.io/crates/sdforge) [![docs.rs](https://img.shields.io/docsrs/sdforge?style=for-the-badge)](https://docs.rs/sdforge) [![downloads](https://img.shields.io/crates/d/sdforge?style=for-the-badge)](https://crates.io/crates/sdforge) [![CI](https://img.shields.io/github/actions/workflow/status/Kirky-X/sdforge/ci.yml?branch=main&label=CI&style=for-the-badge)](https://github.com/Kirky-X/sdforge/actions/workflows/ci.yml) [![coverage](https://img.shields.io/codecov/c/github/Kirky-X/sdforge?style=for-the-badge)](https://codecov.io/gh/Kirky-X/sdforge) [![license](https://img.shields.io/crates/l/sdforge?style=for-the-badge)](LICENSE)

[中文](README.md) | **English**

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
- [🎨 Feature Flags](#-feature-flags)
- [📚 Documentation](#-documentation)
- [💻 Examples](#-examples)
- [🏗️ Architecture](#️-architecture)
- [🧪 Testing](#-testing)
- [📊 Performance](#-performance)
- [🔒 Security](#-security)
- [🗺️ Roadmap](#️-roadmap)
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
<td width="50%" style="vertical-align:top; padding: 12px">🛡️ <b>Security features</b><br><span style="color:#64748B">API Key / JWT Bearer auth, limiteron rate limiting, audit logging, security headers</span></td>
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

Parameter validation, declarative pagination, ETag conditional requests, lifecycle hooks, hook pipeline, graceful shutdown, HTTP TLS termination, health probes, Prometheus metrics, OTel export, request context, response timestamps, structured logging, inklog bridge, trait-kit integration, and SIMD JSON are each enabled on demand via an individual feature; per-item descriptions and defaults are in the [Feature Flags](#-feature-flags) section.

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
sdforge = { version = "0.5.0-rc.9", features = ["http"] }
```

</details>

Minimum requirements:

- **Rust 1.97.1+** (edition 2024, toolchain pinned by `rust-toolchain.toml`)
- **protoc**: only needed when the `grpc` feature is enabled (`build.rs` compiles the protobuf)

> `sdforge` enables no features by default (`default = []`); enable protocol features explicitly as needed.

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

### 🧭 Core concepts

- A `#[forge]` annotation describes one set of endpoint metadata (name, version, path, method, description); the macro generates registration code per enabled feature, submitted at compile time via `inventory::submit!()` and collected once at startup by `init_all_plugins()`
- Pick the entrypoint per protocol: `http::build()` / rmcp stdio / `SdForgeGrpcService` / `CliBuilder::execute()`

### 🔧 `#[forge]` macro parameters

`#[forge]` requires `name` and `version`; common optional parameters include `path` / `method` / `status` / `description` / `tool_name` / `grpc_method` / `cli`. The full parameter table (including advanced parameters and defaults) is in the "`#[forge]` parameters" section of the [API Reference](docs/API_REFERENCE.md#forge-参数).

### 🌐 Protocol combinations

| Goal | features | Scenario |
|------|----------|----------|
| HTTP only | `["http"]` | Traditional REST APIs |
| MCP only | `["mcp"]` | AI tool integration |
| HTTP + MCP dual protocol | `["http", "mcp"]` | One codebase, two entrypoints |
| Full runtime features | `["full"]` | All protocols and capabilities (26 features, incl. `idempotency` and deployment-surface HTTP TLS `serve-tls`/`tls-watch`; excludes the gRPC-side `grpc-tls` and the optional heavy deps / opt-ins `simd-json`/`kit`/`limiteron-integration`/`schemars`/`ratelimit-dist`/`cache-l2`/`db-integration`/`sdk`) |

`grpc`, `websocket`, `streaming`, `openapi`, `cli`, and `cache` can all be enabled independently of `http`, in any combination.

---

## 🎨 Feature Flags

`default = []`: every feature is optional and enabled explicitly.

<details>
<summary>📋 Table</summary>

<table>
  <tr><th>Flag</th><th>Description</th><th>Default</th></tr>
  <tr><td><code>http</code></td><td>HTTP server (Axum 0.8 routing, Tower middleware, version routing)</td><td>❌</td></tr>
  <tr><td><code>mcp</code></td><td>MCP protocol (rmcp 3.5, 2026-07-28 spec: stateless HTTP headers, MRTR, cache semantics)</td><td>❌</td></tr>
  <tr><td><code>grpc</code></td><td>gRPC (tonic + prost, independent of http, proto generated by build.rs; unary <code>Call</code> + server-streaming <code>CallStream</code> — the latter also needs <code>streaming</code>)</td><td>❌</td></tr>
  <tr><td><code>grpc-tls</code></td><td>gRPC TLS wiring (exposes tonic <code>ServerTlsConfig</code>; certificate loading is the caller's responsibility; implemented independently of HTTP-side <code>serve-tls</code> termination and likewise excluded from <code>full</code>)</td><td>❌</td></tr>
  <tr><td><code>websocket</code></td><td>WebSocket (requires http + streaming)</td><td>❌</td></tr>
  <tr><td><code>streaming</code></td><td>SSE streaming (independent of http; combined with <code>grpc</code>, <code>grpc_method</code> + <code>stream = true</code> maps to the gRPC <code>CallStream</code> RPC)</td><td>❌</td></tr>
  <tr><td><code>cli</code></td><td>CLI integration (clap, independent of http)</td><td>❌</td></tr>
  <tr><td><code>openapi</code></td><td>OpenAPI 3.1 spec generation (utoipa, independent of http)</td><td>❌</td></tr>
  <tr><td><code>schemars</code></td><td>Return-type schema reflection (implies openapi: <code>#[forge]</code> emits field-precise response schemas for return types deriving <code>JsonSchema</code>, silently degrading to coarse mapping otherwise)</td><td>❌</td></tr>
  <tr><td><code>docgen</code></td><td>Unified docs output (Swagger UI + CLI/MCP Markdown; requires openapi + cli; legacy name <code>docs</code> remains as a compat alias. Swagger UI v5.32.6 assets are self-vendored under <code>assets/swagger-ui/</code> with zero extra dependencies)</td><td>❌</td></tr>
  <tr><td><code>sdk</code></td><td>Multi-protocol client SDK generation (Rust zero-dep / optional reqwest + TypeScript fetch with type definitions; keeps the CLI <code>sdk</code> subcommand; requires openapi + cli)</td><td>❌</td></tr>
  <tr><td><code>security</code></td><td>Auth (API Key / JWT Bearer), audit, security headers, rate limiting and caching (includes http + ratelimit-http + cache)</td><td>❌</td></tr>
  <tr><td><code>ratelimit</code></td><td>Rate limiting core (limiteron, no http dependency)</td><td>❌</td></tr>
  <tr><td><code>ratelimit-http</code></td><td>HTTP rate limiting middleware (Tower Layer; requires http + ratelimit)</td><td>❌</td></tr>
  <tr><td><code>ratelimit-dist</code></td><td>Distributed rate limiting (limiteron distributed counting backends, quota shared across replicas; unreachable-backend fail-open/fail-close is explicit config, fail-open by default)</td><td>❌</td></tr>
  <tr><td><code>cache</code></td><td>oxcache in-memory cache (independent of http)</td><td>❌</td></tr>
  <tr><td><code>cache-l2</code></td><td>oxcache Redis L2 cache (cross-replica shared cache layer, implies cache; inherently fail-open on failure: read miss / write skip)</td><td>❌</td></tr>
  <tr><td><code>idempotency</code></td><td>Idempotent replay protection (<code>Idempotency-Key</code> HTTP middleware + gRPC metadata path; requires http + cache)</td><td>❌</td></tr>
  <tr><td><code>health</code></td><td><code>/healthz</code> and <code>/readyz</code> probes (auto-mounted by <code>build_with_config</code>, auth bypassed)</td><td>❌</td></tr>
  <tr><td><code>metrics</code></td><td>Prometheus text-format <code>/metrics</code> endpoint (request counts, latency histograms, status code distribution; lightweight in-house renderer)</td><td>❌</td></tr>
  <tr><td><code>graceful</code></td><td>Graceful shutdown (SIGTERM/SIGINT: stop accepting, drain in-flight, phased teardown)</td><td>❌</td></tr>
  <tr><td><code>serve-tls</code></td><td>HTTP TLS termination (rustls aws-lc-rs: PEM cert/key loading, configurable ALPN, <code>ReloadingTls</code> hot reload; per-request <code>ConnectInfo</code> injection, handshake/header-read timeout guards, shutdown choreography sharing graceful's stop phase. Doc-cross-linked with the gRPC-side <code>grpc-tls</code> but implemented independently)</td><td>❌</td></tr>
  <tr><td><code>tls-watch</code></td><td>Debounced certificate-watch hot reload (confers <code>FsWatcher</code> drives <code>reload_async</code> on cert/key file changes; implies <code>serve-tls</code>; keeps the previous certificate on failed reload)</td><td>❌</td></tr>
  <tr><td><code>context</code></td><td>Request context (request_id/trace_id generation and cross-protocol HTTP/MCP/gRPC/WS injection)</td><td>❌</td></tr>
  <tr><td><code>validate</code></td><td><code>#[forge(validate)]</code> + <code>#[param(ge/le/...)]</code> parameter validation; 422 with field-level errors; struct query parameters extracted flatly via <code>#[param(kind = "query", flatten)]</code></td><td>❌</td></tr>
  <tr><td><code>paginate</code></td><td><code>#[forge(paginate)]</code> declarative pagination (auto page/size and the <code>{items,total,next}</code> wrapper)</td><td>❌</td></tr>
  <tr><td><code>etag</code></td><td>ETag conditional requests (automatic SHA-256 strong ETag on GET; If-None-Match returns 304)</td><td>❌</td></tr>
  <tr><td><code>lifecycle</code></td><td><code>#[forge(on_start/on_stop)]</code> lifecycle hooks (ordered with graceful shutdown)</td><td>❌</td></tr>
  <tr><td><code>hooks</code></td><td>Pre/post handler hook pipeline (middleware-style, unified error contract)</td><td>❌</td></tr>
  <tr><td><code>otel</code></td><td>OTLP/HTTP JSON export of request spans and metric snapshots (zero extra dependencies)</td><td>❌</td></tr>
  <tr><td><code>timestamp</code></td><td>Response timestamps</td><td>❌</td></tr>
  <tr><td><code>inklog</code></td><td>inklog structured logging bridge (<code>init_inklog_logger()</code>). Since 0.5.0-rc it absorbs the removed in-house <code>StructuredLogger</code> stack (the <code>logging</code> feature is gone, breaking)</td><td>❌</td></tr>
  <tr><td><code>i18n</code></td><td>ICU4X internationalization (locale-aware formatting + Accept-Language parsing)</td><td>❌</td></tr>
  <tr><td><code>simd-json</code></td><td>SIMD-accelerated JSON serialization/deserialization</td><td>❌</td></tr>
  <tr><td><code>limiteron-integration</code></td><td>Pulls in the limiteron dependency (foundation for kit)</td><td>❌</td></tr>
  <tr><td><code>db-integration</code></td><td>dbnexus data-API gateway (<code>DbGateway</code> whitelisted read-only data plane: table/column whitelists + filter-value escaping + server-side page clamping; independent of http)</td><td>❌</td></tr>
  <tr><td><code>kit</code></td><td>trait-kit AsyncKit integration (SdforgeModule graph)</td><td>❌</td></tr>
  <tr><td><code>tokio</code></td><td>Internal feature: enables the tokio dependency (pulled in automatically by other features)</td><td>❌</td></tr>
  <tr><td><code>full</code></td><td>All runtime features (26: http/mcp/grpc/websocket/streaming/security/cache/health/metrics/graceful/context/validate/paginate/etag/hooks/lifecycle/otel/timestamp/openapi/cli/docgen/inklog/i18n/idempotency/serve-tls/tls-watch; excludes the gRPC-side <code>grpc-tls</code> and the optional heavy deps / opt-in surface <code>simd-json</code>/<code>kit</code>/<code>limiteron-integration</code>/<code>schemars</code>/<code>ratelimit-dist</code>/<code>cache-l2</code>/<code>db-integration</code>/<code>sdk</code>)</td><td>❌</td></tr>
</table>

</details>

<details>
<summary>🔗 Feature dependency relations</summary>

- Independent of `http`: `mcp` / `grpc` / `openapi` / `cli` / `streaming` / `cache` / `timestamp` / `context` / `inklog` / `i18n` / `simd-json` / `limiteron-integration` / `db-integration` / `schemars` / `ratelimit-dist`
- Derived from `http`: `security` (includes `ratelimit-http` → `ratelimit` and `cache`), `ratelimit-http`, `websocket` (includes `streaming`), `health` / `metrics` / `graceful` / `serve-tls` (includes `graceful`) / `tls-watch` (includes `serve-tls`) / `validate` / `paginate` / `lifecycle` / `hooks` / `otel`
- `docgen` = `openapi` + `cli` (mounting the Swagger UI additionally requires `http`; legacy name `docs` remains as a compat alias)
- `kit` = `trait-kit` (health + lifecycle) + `limiteron-integration` + `limiteron/kit` + `oxcache/kit`
- `full` covers 26 runtime features (incl. `idempotency`, `docgen`, deployment-surface HTTP TLS `serve-tls`/`tls-watch`) and excludes the optional heavy deps `simd-json` / `kit` / `limiteron-integration` (SIMD JSON, trait-kit module graph, rate-limit integration foundation) plus the gRPC-side `grpc-tls` wiring and `schemars` / `ratelimit-dist` / `cache-l2` / `db-integration` / `sdk` (opt-in surface) — all enabled individually as needed

> ℹ️ **`docgen` availability note** (the 2026-10-16 upstream `zip` yank): that incident broke resolution of `docgen`'s dependency `utoipa-swagger-ui` (pinned to `zip ^8.6`), so enabling `docgen` from scratch (including `full`, which covers it) failed. The dependency is now removed: the Swagger UI v5.32.6 official dist subset (Apache-2.0, modifications documented in the asset `NOTICE.md`) is self-vendored under `assets/swagger-ui/`, embedded at compile time via `include_bytes!`, and served directly by `swagger_ui_router*`; `docgen` is now just `openapi + cli`, so downstream dependency resolution works again. Refresh assets with `scripts/update_swagger_ui.sh`.

</details>

---

## 📚 Documentation

<details>
<summary>📋 Table</summary>

| Document | Description |
|----------|-------------|
| [📖 User Guide](docs/USER_GUIDE.md) | Complete tutorial from installation and core concepts to advanced usage |
| [📘 API Reference](docs/API_REFERENCE.md) | Public APIs of the core and every feature gate |
| [🏗️ Architecture](docs/ARCHITECTURE.md) | Design principles, module breakdown, data flow, security and performance design |
| [⚡ Performance Baselines](docs/PERFORMANCE.md) | Runtime hot-path criterion baselines and regression guidelines |
| [📶 Compile-Time Gating Benchmarks](docs/benchmarks/VS_SERVER_LESS.md) | Compile time and artifact size: feature gating vs full build |
| [🔒 Security](docs/SECURITY.md) | Vulnerability reporting process, security design, and best practices |
| [🧾 Test Scenarios](docs/TEST_SCENARIOS.md) | Test pyramid baseline and E2E scenario definitions |
| [📋 Changelog](docs/CHANGELOG.md) | Change log for every release |
| [🤝 Contributing](docs/CONTRIBUTING.md) | Development environment, TDD workflow, and PR process |
| [📦 Online API Docs](https://docs.rs/sdforge) | Latest docs.rs documentation (all features) |

</details>

---

## 💻 Examples

### Root package examples (`cargo run --example`)

<details>
<summary>📋 Table</summary>

| Example | Required features | Description |
|---------|-------------------|-------------|
| `basic_cli` | `cli` | `#[forge(cli = true)]` + one-shot `CliBuilder::execute()` CLI |
| `swagger_demo` | `docgen` + `http` | Swagger UI route + OpenAPI JSON + axum serve |
| `perf_regex_cache` | `cache` | Cache key wildcard matching (`find_keys_by_pattern`): `*` / `?` correctness and per-call full-scan cost |
| `perf_lru_eviction` | `cache` | Capacity eviction (`with_capacity` FIFO insertion order: oldest-first, reads do not refresh a key) |
| `perf_prefix_index` | `cache` | Prefix index performance verification |
| `perf_batch_ops` | `cache` | Batch operation performance verification |

</details>

<details>
<summary>📄 Code · bash</summary>

```bash
cargo run --example basic_cli --features cli -- echo --name world
cargo run --example swagger_demo --features "docgen http"
cargo run --example perf_regex_cache --features cache
```

</details>

### Comprehensive example library (workspace member `sdforge-examples`, `examples/src/`)

<details>
<summary>📋 Table</summary>

| Module | Contents |
|--------|----------|
| `basics/` | Simple API, response building, types and error handling |
| `http/` | Routing (path params, query params), middleware (CORS) |
| `mcp/` | Tool definition & registration, MCP 2026-07-28 migration, MRTR sessions |
| `security/` | API Key auth, auth failure scenarios, full security stack |
| `cache/` | Advanced caching patterns and performance verification |
| `config/` | Configuration management (`app_config.rs`) |
| `streaming/` | SSE streaming responses |
| `websocket/` | Basic usage and chat room |
| `grpc/` | gRPC server |
| `logging/` | Structured logging (inklog LoggerManager: global install / from_file rotation / builder / set_level) |
| `openapi/` | OpenAPI spec generation (`OpenApiBuilder`, `generate_openapi_spec`) |
| `combined/` | Complete example combining multiple features (`full_example.rs`) |

</details>

### Ecosystem integration examples (`sdforge-examples` member examples)

| Example | Feature | Description |
|---------|---------|-------------|
| `oxcache_admin` | `oxcache_admin_example` | Expose oxcache management endpoints via `BackendRegistry` |
| `dbnexus_gateway` | `dbnexus_gateway_example` | Read-only data API gateway over a whitelisted table (in-memory sqlite) |

<details>
<summary>📄 Code · bash</summary>

```bash
cargo run -p sdforge-examples --example oxcache_admin --features oxcache_admin_example
```

</details>

Sample configuration files live in `examples/config/` (`default.toml`, `minimal.toml`, `production.toml`, `api-key-auth.toml`).

---

## 🏗️ Architecture

SDForge consists of two crates: `macros/sdforge-macros` parses the `#[forge]` / `#[service_module]` annotations and generates feature-gated registration code, while `sdforge` is the runtime library. Registrations are submitted at compile time via `inventory::submit!()` and consolidated at startup by `init_all_plugins()`; the protocol modules (http / mcp / grpc / websocket / streaming / cli) are mutually independent and compiled per feature, all sharing the unified handler contract in `core` (`HandlerArgs` + `HandlerState`). The full module-registration map, repository layout, and design rationale are in the [Architecture document](docs/ARCHITECTURE.md).

### Design principles

The five design principles — compile-time protocol selection, the inventory registration pattern, the unified handler contract, three construction modes (`new()` / `builder()` / `with_dependencies()`), and no database — are described in full in the [Architecture document](docs/ARCHITECTURE.md#-设计原则).


### 🔄 Core Execution Path

Taking the HTTP request hot path as an example: a request passes the middleware stack (authentication → rate limiting → security headers / CORS), the version router matches `/api/{version}`, the forge handler runs the business logic under the unified handler contract (`HandlerArgs` + `HandlerState`), and the `ServiceResponse` pipeline emits the JSON response; the MCP, gRPC, and CLI entrypoints reuse the same handlers and response contract, with middleware order, version routing, and the response pipeline assembled by `http::build()` / `build_with_config()`. The full sequence diagram and data flow are in the [Architecture document](docs/ARCHITECTURE.md#-数据流).


### 🌐 One Annotation, Five Protocols

The `#[forge]` macro generates protocol registrations for the features currently enabled (HTTP / MCP / gRPC / CLI / OpenAPI); disabled protocols generate no code at all. MCP routes via the `Mcp-Method` / `Mcp-Name` headers (or stdio), gRPC dispatches via `SdForgeGrpcService::call()` keyed by `grpc_method`, CLI runs parsing, dispatch, and exit codes through `CliBuilder::execute()`, and protocols are decoupled at runtime. The compile-time registration flow and request-phase data flow are detailed in the [Architecture document](docs/ARCHITECTURE.md#-数据流).

---

## 🧪 Testing

### Test strategy matrix

<details>
<summary>📋 Table</summary>

| Layer | Location | Description |
|-------|----------|-------------|
| Unit tests | Embedded `#[cfg(test)]` in `src/`, `tests/unit/` | Module-level tests, including proptest property tests (`src/tests/property_tests.rs`) |
| Integration tests | `tests/integration/` | Covers http / mcp / grpc / websocket / security / cache / streaming / openapi / cli / docs / health_probes / graceful_shutdown / http_tls / validate / paginate / etag / lifecycle / otel_export / status_code / rbac protocol and feature combinations |
| Macro tests | `tests/macros/`, `macros/tests/` | trybuild compile-failure cases and macro expansion verification |
| E2E | `tests/e2e/` | `e2e_advanced` multi-domain E2E scenarios (11-domain scale baseline in [Test Scenarios](docs/TEST_SCENARIOS.md)) |
| Examples comprehensive tests | `examples/tests/` | All-feature re-export accessibility and cross-protocol dispatch (76 tests) plus gateway E2E |
| Benchmarks | `benches/`, `src/benches/` | criterion: `runtime_bench` / `config_and_cache_bench` / `sdforge_bench` |
| Doc-tests | `src/` doc comments | Embedded rustdoc examples |

</details>

### Running commands

<details>
<summary>📄 Code · bash</summary>

```bash
# Aligned with the CI matrix (http / mcp / http,mcp / http,security / http,cache /
# http,websocket / http,grpc / http,streaming / full — 9 combinations).
# The matrix tests the sdforge package only (-p sdforge): --workspace triggers
# feature unification through sdforge-examples' default features, which pulls
# nearly every sdforge feature into each combination.
cargo test -p sdforge --features "http,mcp"

# Full workspace (sdforge + sdforge-macros + sdforge-examples, all features)
cargo test --workspace --all-features

# Lib tests (the CI coverage target)
cargo test --features full --lib

# Coverage (CI gate >=90% line coverage: --features full --lib; lefthook pre-push applies the same threshold without --features full --lib)
cargo llvm-cov --features full --lib --lcov --fail-under-lines 90

# Formatting and zero-warning lint
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

</details>

The build.rs-generated protobuf code goes to `OUT_DIR` and no longer enters the source tree; the `src/grpc/pb/*` exclusion in `llvm-cov.toml` is a leftover from the old layout and currently matches no files.

### Test scale

About **3,181** test functions (`src/` 2,180 + `tests/` 785 + `macros/` 92 + `examples/` 124; via `grep -rEc '#\[(tokio::)?test\b' --include='*.rs'`, re-verified 2026-10-11).

---

## 📊 Performance

### Runtime hot path (criterion baselines)

`plain_get` routing dispatch runs at a median of ~553 ns (~1.81 M req/s), ~604 ns with 1 path parameter; the full baselines for HandlerArgs assembly and JSON serialization/deserialization, environment details, and reproduction commands are in [Performance Baselines](docs/PERFORMANCE.md).

### Compile-time gating gains (feature gating vs full build)

`http` only saves ~47% compile time, roughly 60% of framework rlib size, and 82 unique dependency crates compared to `full`; the full data and methodology are in the [compile-time gating benchmarks](docs/benchmarks/VS_SERVER_LESS.md) (measured 2026-07-03 on AMD Ryzen 9 9950X / rustc 1.93.1 / WSL2).

---

## 🔒 Security

### 🚨 Reporting vulnerabilities

**Do not report security vulnerabilities through public GitHub issues.** Please use the private GitHub [Security Advisories](https://github.com/Kirky-X/sdforge/security/policy) disclosure channel; response timelines and the process are in [docs/SECURITY.md](docs/SECURITY.md).

### 🛡️ Security design highlights

The design rationale for authentication (API Key / JWT Bearer with enforced key length), fail-safe defaults (binding `127.0.0.1` by default), unspoofable client IPs, audit signing, error sanitization, and input defense (MCP schema validation and 1 MiB payload caps) is in the [Architecture document](docs/ARCHITECTURE.md#-安全设计) and [docs/SECURITY.md](docs/SECURITY.md).

### 🔍 Supply chain security

CI security gates are always on: `cargo deny check` ([deny.toml](deny.toml) policy) + `cargo audit`, together with CodeQL, Dependabot, and pre-commit secret scanning (detect-secrets); releases additionally require the SAST scan and code review gates — see the release process in the [Contributing Guide](docs/CONTRIBUTING.md).

---

## 🗺️ Roadmap

<details>
<summary>📋 Table</summary>

<table>
  <tr><th>Status</th><th>Item</th><th>Notes</th></tr>
  <tr><td>🚧</td><td><b>v0.5.0 release</b></td><td>Currently on the <code>0.5.x</code> release line; completing coordinated releases of the dependency chain (trait-kit / oxcache / inklog / limiteron) and final verification</td></tr>
  <tr><td>✅</td><td>Custom success status codes</td><td><code>#[forge(status = &lt;code&gt;)]</code> static declaration + <code>ServiceResponse::success_with_status</code> dynamic control, released in 0.5.0-rc.2</td></tr>
  <tr><td>✅</td><td>MSRV alignment</td><td>Workspace unified to 1.97.1 (2026-09-06), covering the effective requirement under <code>--all-features</code></td></tr>
  <tr><td>✅</td><td>Unified error-code behavior contract</td><td>Contract recorded (<code>docs/API_REFERENCE.md</code> error-code contract table + <code>error::unified</code> single source of truth): HTTP 400 (malformed syntax/<code>InvalidInput</code>) vs 422 (semantic violation/<code>ValidationError</code>) distinction retained; gRPC has no native 422 counterpart, so both map to <code>invalid_argument</code> with the semantic code carried in the <code>Status::details</code> payload <code>code</code> field; pinned by the <code>grpc_code_for_http_status</code> contract table + cross-protocol consistency tests (full-universe JOIN invariant + wire-level e2e)</td></tr>
  <tr><td>✅</td><td>Dependency hygiene</td><td><code>bincode</code> (RUSTSEC-2025-0141 unmaintained) fully migrated to <code>postcard</code> (equivalent replacement for API-key metadata/permission serialization); stale <code>deny.toml</code> exemption removed; <code>cargo deny</code> / <code>cargo audit</code> re-verified clean</td></tr>
</table>

</details>

---

## 🤝 Contributing

Contributions are welcome! Development environment setup (toolchain, protoc, lefthook / pre-commit hooks), the TDD workflow, feature-combination checks, and the Conventional Commits message convention (enforced by the commit-msg hook) are described in the [Contributing Guide](docs/CONTRIBUTING.md).

---

## 📋 Changelog

See [CHANGELOG.md](docs/CHANGELOG.md). Highlights of recent releases:

- **[0.5.0-rc.8]** (2026-10-06): MCP auth propagation across call_tool and CLI entry, `AuthConfig` fail-closed without the security feature; multi-protocol contract parity — Idempotency-Key replay protection, RBAC/auth across gRPC, unified error mapping, validation unified to 422; async health checks and the `ReadinessRenderer` port; external OpenAPI spec merging and vendored Swagger UI (zero network at build time); knowledge-pack CLI (`--format json` / `docs --format agent`); post-drain graceful hooks; three rounds of security review fixes
- **[0.5.0-rc.5]** (2026-09-21): Maintenance release — `forge` status=204/304 bodyless responses, validator crate re-export, custom Swagger UI spec URL, i18n overhaul, `SdForge` naming unification, cross-repo path deps moved to crates.io and hardening
- **[0.5.0-rc.4]** (2026-09-14, including the original rc.3 batch): `ResponseCacheLayer` response caching middleware, `SdForgeConfig` security/cache fields, `AuditSink` abstraction and `InklogAuditSink` (version 0.5.0-rc.3 was skipped and never published; its content shipped with this release)
- **[0.5.0-rc.2]** (2026-09-07): `#[forge(status = <code>)]` custom success status codes, `i18n_key` parameter and translation registry, rmcp 2.2 → 3.2
- **[0.4.7]** (2026-07-23): Removed tilde constraints from dependency versions; published the `bincode` RUSTSEC-2025-0141 ignore decision

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

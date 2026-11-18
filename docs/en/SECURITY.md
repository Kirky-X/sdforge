# 🔒 Sdforge Security

This document describes SDForge's security support policy, vulnerability reporting process, security design overview, and security best practices. SDForge is a multi-protocol SDK framework and security is one of its core design goals: the `security` feature provides out-of-the-box authentication, rate limiting, auditing, and security headers, while the framework itself minimizes its attack surface through compile-time feature gating.

## 📋 Table of Contents

<details open>
<summary>📑 Table of Contents</summary>

- [Supported Versions and Maintenance Policy](#-supported-versions-and-maintenance-policy)
- [Vulnerability Reporting Process](#-vulnerability-reporting-process)
- [Security Design Overview](#-security-design-overview)
- [Security Best Practices](#-security-best-practices)

</details>

## 📌 Supported Versions and Maintenance Policy

The project is in its 0.x pre-release phase and **maintains only the current minor line** (`0.5.x`, including rc pre-releases on that line).

- **Current line**: receives bug fixes and security fixes, shipped with the latest release on that line.
- **Older minor lines**: enter EOL when the first version of the next minor line ships; no further fixes of any kind, and security patches are not backported.
- **No LTS**: before 1.0 there is no cross-minor API compatibility promise; minor-version upgrades may contain breaking changes — read the [CHANGELOG](CHANGELOG.md) before upgrading.
- **Visible on crates.io ≠ supported**: old versions remain downloadable (unless yanked for a severe defect) but are outside the supported set.

| Version line | Status | Notes |
|--------|------|------|
| `0.5.x` (current) | ✅ Supported | The only line receiving fixes |
| `≤ 0.4.x` | ❌ EOL | Superseded by the current line, unmaintained — please upgrade |

## 🚨 Vulnerability Reporting Process

We take security vulnerabilities seriously. If you discover a security vulnerability in SDForge, please report it responsibly.

### How to Report

**Do not open a public GitHub Issue for security vulnerabilities.**

**Preferred channel**: submit a report via the GitHub Security Advisories private disclosure channel, [Report a vulnerability](https://github.com/Kirky-X/sdforge/security/policy).
**Fallback channel**: only if the preferred channel is unavailable, email **Kirky-X@outlook.com** (contact: Kirky.X).

Please include the following in your report:

1. Description of the vulnerability
2. Reproduction steps
3. Potential impact
4. Suggested fix (if any)

### ⏱️ Response Commitments

- **Acknowledgment**: within 48 hours
- **Initial assessment**: within 7 days
- **Fix or mitigation**: depending on severity; Critical issues are usually addressed within 30 days

### Disclosure Policy

- We follow responsible disclosure practices
- We credit reporters in the release notes (unless anonymity is requested)
- Please do not publicly disclose the vulnerability before the fixed version is released

## 🛡️ Security Design Overview

### Security feature layering (`security-core` / `security-http` / `security`)

`security` splits into two layers so non-HTTP protocols can use authentication without pulling in the HTTP stack:

| Feature | Contents | Pulls in HTTP? |
|------|------|------|
| `security-core` | Protocol-neutral core: credential stores (`BearerAuth`, `SdForgeApiKeyAuth`, API-key manager), audit, `ProtocolAuthVerifier` port and `make_verifier(&AuthConfig)` | No |
| `security-http` | HTTP adapters: `auth_middleware`, client-IP extraction, endpoint RBAC (`require_role`), security headers, HTTP rate-limit wiring | Yes (depends on `security-core`) |
| `security` | Backward-compatible aggregate alias = `security-core` + `security-http` | Yes (identical closure to pre-split `security`) |

A pure MCP / gRPC / CLI project enables `security-core` together with its protocol feature (for example `mcp,security-core`); no axum/tower is compiled. The `security` alias keeps existing projects unchanged. Rate limiting remains an independent layer (`ratelimit` core / `ratelimit-http`); `security-core` does not imply `ratelimit`.

### Built-in Security Features (`security` feature)

| Capability | Description |
|------|------|
| Authentication | API Key and JWT Bearer Token authentication (`ApiKeyAuth` / `BearerAuth`, with the `auth_middleware` middleware). Non-HTTP entrypoints reuse the same credential store: the gRPC interceptor and MCP `call_tool`/`list_tools`, CLI `execute` go through the `GrpcAuthVerifier` port (recommended alias `ProtocolAuthVerifier`, with `make_verifier(&AuthConfig)` as the unified construction point; each entrypoint is **wired manually**); the WS handshake validates via `auth`/`api_key_auth` embedded in `WebSocketConfig` (same credential store, separate path, not through the port). MCP credentials are injected by the transport adapter layer into JSON-RPC extensions via `McpCredentials::from_headers` — without injection, every gated call is rejected (the built-in stdio transport cannot carry credentials). **Coverage boundary: CLI authenticates but does not authorize** — `#[forge(auth(role = ...))]` takes effect on HTTP/gRPC/MCP (MCP returns JSON-RPC `-32003` when no role matches), and does not apply to CLI/WS (see the protocol security coverage matrix below) |
| Rate limiting | limiteron-based per-connection / per-IP rate limiting (`ratelimit` core and the `ratelimit-http` Tower middleware) |
| Security headers | Standard security response headers (CORS, CSP, etc., `SecurityHeaders`) |
| Audit log | Security event audit trail (`AuditLogger`, HMAC-SHA256 tamper-proof signing) |
| Input validation | Thorough input validation (email, length, etc.) |

### Protocol Security Coverage Matrix

| Capability | HTTP | gRPC | MCP | CLI | WS |
|------|------|------|-----|-----|-----|
| Authentication (credential verification) | ✅ middleware | ✅ interceptor | ✅ `call_tool`/`list_tools` | ✅ `execute` | ✅ handshake |
| Authorization (RBAC roles) | ✅ `require_role` | ✅ per-method roles | ✅ per-tool roles (`call_tool`/`call_tool_with_credentials`, no role permission → `-32003`; fail-safe deny-all when `security` is off) | ❌ | ❌ |
| Parameter validation | ✅ `#[forge(validate)]` | ✅ equivalent | ✅ input schema | — | — |

> **The coverage boundary is registered security debt, not a final state**: authorization (`#[forge(auth(role = ...))]`/roles) is not yet implemented for CLI/WS — CLI sits inside a local trust boundary (the process entry point is the operator), and after the WS handshake there is no per-request identity channel; before introducing network-triggered forms, authorization must be added or compensated at the deployment layer (network isolation / least-privilege credentials). Pre-release checks must verify this matrix against the implementation.

### Security Defaults (tightened since v0.3.0)

- **JWT secret minimum length**: `MIN_SECRET_LENGTH=32`; secrets shorter than 32 characters are rejected
- **ServerConfig default host**: `DEFAULT_HOST` changed from `"0.0.0.0"` (fail-open) to `"127.0.0.1"` (fail-safe loopback); without explicit configuration nothing is exposed on all interfaces
- **Tightened CORS validation**: `"http://"` (scheme only, no host) is rejected in both `validate()` and `build_cors_layer()`
- **AuthConfig fail-closed**: when `AuthConfig::ApiKey`/`Jwt` is configured but the `security` feature is not enabled, `SdForgeConfig::validate()` and `build_with_config_ext()` fail loudly at build time (the auth middleware as a whole is provided by the `security` feature; previously this combination was silently ignored, producing an unauthenticated router); `AuthConfig::None` without the `security` feature remains a valid combination

### Key Security Fixes (historical disclosures)

- **Client IP extraction (v0.4.4)**: `extract_client_ip_core` no longer last-resort trusts `X-Forwarded-For` / `X-Real-IP` headers without `ConnectInfo`, returning `None` instead; this closes the vector where IP rate limiting/banning could be bypassed with spoofed headers in deployments without `ConnectInfo`
- **Payload size cap (v0.4.3)**: the gRPC `call` path gained a 1 MiB argument payload cap, aligned with MCP `MAX_ARGUMENTS_SIZE_BYTES`, closing the oversized-payload DoS vector
- **MCP schema validation (v0.4.4)**: fixed spurious extra quotes in the `required` field of the `input_schema` generated by the `#[forge]` macro, making required / unknown-field validation actually effective
- **API Key empty-store fail-loud**: the `AuthConfig::ApiKey.keys` seeding mechanism fails loudly on an empty store instead of silently letting everything through
- **Error sanitization**: `ApiError::Internal.message` is sanitized before output (no paths/stack traces), and `error_id` supports log-side correlation; concurrency vulnerabilities (cache races, mutex poisoning, rate-limit underflow, etc.) were fixed systematically in the v0.3.0 security audit

### Dependency Security Monitoring

We monitor dependencies through:

| Tool | Purpose |
|------|------|
| cargo-audit | Scans dependencies for known vulnerabilities |
| cargo-deny | Enforces license and vulnerability policy (`deny.toml`) |
| Dependabot | Automated dependency update PRs |
| CodeQL | Semantic code security analysis |

The framework's internal security design trade-offs are in the security design section of the [architecture document](ARCHITECTURE.md); where usage-side best practices (`ConnectInfo` configuration, API Key rotation, etc.) overlap with the [user guide](USER_GUIDE.md) best-practices section, this document prevails.

## ✅ Security Best Practices

1. **Configure the host explicitly for production** — `ServerConfig::default()` binds `127.0.0.1` (fail-safe); public services must configure the listen address explicitly
2. **Use strong secrets** — JWT secrets are at least 32 characters (enforced by the framework)
3. **Configure `ConnectInfo`** — production deployments must configure axum `with_make_service_with_connect_info` to enable unspoofable TCP peer IP extraction (rate limiting/banning depends on it)
4. **Enable features on demand** — use compile-time protocol selection to trim unused protocols (e.g. do not enable `full` when you only need HTTP) to minimize attack surface and build artifacts
5. **Define an API Key rotation policy** — the security module supports API Key version management and audited key rotation; seed explicitly via `AuthConfig::ApiKey.keys`
6. **Enable audit log signing** — follow the HMAC-SHA256 tamper-proof signing practice in `examples/src/security/comprehensive.rs`
7. **Reference the example configurations** — `examples/config/api-key-auth.toml` (API Key auth) and `examples/config/production.toml` (production configuration)
8. **Watch dependency advisories** — the project monitors advisories continuously via `cargo deny check`; exemption decisions (if any) are disclosed transparently in the CHANGELOG — the historical exception for bincode RUSTSEC-2025-0141 ended with the bincode → postcard migration and the exemption was removed

# 🔒 Sdforge 安全文档

本文档描述 SDForge 的安全支持策略、漏洞报告流程、安全设计概览与安全最佳实践。SDForge 是一个多协议 SDK 框架，安全性是其核心设计目标之一：`security` 特性提供认证、限流、审计与安全头等开箱即用的能力，框架本身则通过编译时特性门控最小化攻击面。

## 📋 目录

<details open>

- [支持版本](#-支持版本)
- [漏洞报告流程](#-漏洞报告流程)
- [安全设计概览](#️-安全设计概览)
- [安全最佳实践](#-安全最佳实践)

</details>

## 📦 支持版本

| 版本    | 支持状态           |
|---------|--------------------|
| 0.5.x（含 rc） | :white_check_mark: |
| 0.4.x   | :white_check_mark: |
| < 0.4   | :x:                |

## 🚨 漏洞报告流程

我们高度重视安全漏洞。如果你发现 SDForge 的安全漏洞，请负责任地报告。

### 如何报告

**不要为安全漏洞创建公开的 GitHub Issue。**

请将安全问题通过邮件发送给维护者 **kirky.x@outlook.com**，并包含以下信息：

1. 漏洞描述
2. 复现步骤
3. 潜在影响
4. 修复建议（如有）

### 响应时限

- **确认收到**：48 小时内
- **初步评估**：7 天内
- **修复或缓解**：视严重程度而定，Critical 问题通常在 30 天内

### 披露政策

- 我们遵循负责任披露（responsible disclosure）实践
- 我们将在发布说明中致谢报告者（除非要求匿名）
- 在修复版本发布之前，请勿公开披露该漏洞

## 🛡️ 安全设计概览

### 内置安全特性（`security` feature）

- **认证**：API Key 与 JWT Bearer Token 认证（`ApiKeyAuth` / `BearerAuth`，含 `auth_middleware` 中间件）
- **限流**：基于 limiteron 的按连接 / 按 IP 限流（`ratelimit` 核心与 `ratelimit-http` Tower 中间件）
- **安全头**：标准安全响应头（CORS、CSP 等，`SecurityHeaders`）
- **审计日志**：安全事件审计追踪（`AuditLogger`，支持 HMAC-SHA256 签名防篡改）
- **输入校验**：完善的输入校验（邮箱、长度等）

### 安全默认值（v0.3.0+ 收紧）

- **JWT 密钥最小长度**：`MIN_SECRET_LENGTH=32`，短于 32 字符的密钥被拒绝
- **ServerConfig 默认 host**：`DEFAULT_HOST` 从 `"0.0.0.0"`（fail-open）改为 `"127.0.0.1"`（fail-safe 回环），未显式配置时不会暴露到所有网卡
- **CORS 校验收紧**：`"http://"`（仅 scheme 无 host）在 `validate()` 与 `build_cors_layer()` 中均被拒绝

### 关键安全修复（历史披露）

- **客户端 IP 提取（v0.4.4）**：`extract_client_ip_core` 在无 `ConnectInfo` 时不再 last-resort 信任 `X-Forwarded-For` / `X-Real-IP` 头，直接返回 `None`，消除未配置 `ConnectInfo` 部署下 IP 限流/封禁被伪造头绕过的向量
- **载荷大小上限（v0.4.3）**：gRPC `call` 路径新增 1 MiB 参数载荷上限，与 MCP `MAX_ARGUMENTS_SIZE_BYTES` 对齐，关闭超大载荷 DoS 向量
- **MCP schema 校验（v0.4.4）**：修复 `#[forge]` 宏生成 `input_schema` 的 `required` 字段多余引号问题，使 required / unknown-field 校验真正生效
- **API Key 空库 fail-loud**：`AuthConfig::ApiKey.keys` 播种机制，空库时显式失败而非静默放行
- **错误脱敏**：`ApiError::Internal` 的 message 经过清洗，不泄露路径、堆栈等内部实现细节；并发漏洞（缓存竞态、Mutex 中毒、限流下溢等）已在 v0.3.0 安全审计中系统性修复

### 依赖安全监控

我们通过以下方式监控依赖：

- **cargo-audit**：扫描依赖中的已知漏洞
- **cargo-deny**：强制执行许可与漏洞策略（`deny.toml`）
- **Dependabot**：自动化依赖更新 PR
- **CodeQL**：语义化代码安全分析

## ✅ 安全最佳实践

1. **生产部署显式配置 host** — `ServerConfig::default()` 绑定 `127.0.0.1`（fail-safe），对外服务必须显式配置监听地址
2. **使用强密钥** — JWT 密钥至少 32 字符（框架强制校验）
3. **配置 `ConnectInfo`** — 生产部署必须配置 axum `with_make_service_with_connect_info`，以启用不可伪造的 TCP 对端 IP 提取（限流/封禁依赖它）
4. **按需启用特性** — 利用编译时协议选择裁剪未用协议（如只需 HTTP 时不要启用 `full`），最小化攻击面与编译产物
5. **为 API Key 制定轮换策略** — 安全模块支持 API Key 版本管理与带审计日志的密钥轮换；通过 `AuthConfig::ApiKey.keys` 显式播种
6. **为审计日志启用签名** — 参考 `examples/src/security/comprehensive.rs` 的 HMAC-SHA256 防篡改签名实践
7. **参考示例配置** — `examples/config/api-key-auth.toml`（API Key 认证）与 `examples/config/production.toml`（生产配置）
8. **关注依赖公告** — 项目通过 `cargo deny check` 持续监控公告；已知例外（如 bincode RUSTSEC-2025-0141 unmaintained 的 ignore 决策）会在 CHANGELOG 中透明披露

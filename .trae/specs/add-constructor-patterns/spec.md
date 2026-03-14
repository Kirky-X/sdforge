# 组件构造模式完善 Spec

## Why

根据 AGENTS.md 规范，所有功能组件必须支持三种构造模式：开箱即用 (`new()`)、Builder 模式 (`builder()`)、完全依赖注入 (`with_dependencies()`)。当前 `BearerAuth`、`RateLimiter` 和 `AuditLogger` 组件未完全遵循此规范。

## What Changes

- 为 `BearerAuth` 添加 `builder()` 方法和 `BearerAuthBuilder` 结构体
- 为 `BearerAuth` 添加 `with_dependencies()` 方法
- 为 `RateLimiter` 添加 `builder()` 方法和 `RateLimiterBuilder` 结构体
- 为 `RateLimiter` 添加 `with_dependencies()` 方法
- 为 `AuditLogger` 添加 `builder()` 方法和 `AuditLoggerBuilder` 结构体
- 为 `AuditLogger` 添加 `with_dependencies()` 方法

## Impact

- Affected specs: 安全模块组件构造规范
- Affected code: `src/security.rs`

## ADDED Requirements

### Requirement: BearerAuth 三种构造模式

系统应为 `BearerAuth` 提供完整的三种构造模式支持。

#### Scenario: 开箱即用模式
- **WHEN** 用户调用 `BearerAuth::new(secret)` 或 `BearerAuth::try_new(secret)`
- **THEN** 返回使用默认配置的 BearerAuth 实例

#### Scenario: Builder 模式
- **WHEN** 用户使用 `BearerAuth::builder().secret(s).audience(a).issuer(i).build()`
- **THEN** 返回使用自定义配置的 BearerAuth 实例
- **AND** 构建失败时返回明确的错误信息

#### Scenario: 完全依赖注入模式
- **WHEN** 用户调用 `BearerAuth::with_dependencies(secret, valid_tokens, blacklisted_tokens, audience, issuer)`
- **THEN** 返回使用外部依赖的 BearerAuth 实例

### Requirement: RateLimiter 三种构造模式

系统应为 `RateLimiter` 提供完整的三种构造模式支持。

#### Scenario: 开箱即用模式
- **WHEN** 用户调用 `RateLimiter::new(None)` 或 `RateLimiter::default()`
- **THEN** 返回使用默认配置的 RateLimiter 实例

#### Scenario: Builder 模式
- **WHEN** 用户使用 `RateLimiter::builder().max_requests(100).window(Duration::from_secs(60)).build()`
- **THEN** 返回使用自定义配置的 RateLimiter 实例

#### Scenario: 完全依赖注入模式
- **WHEN** 用户调用 `RateLimiter::with_dependencies(config, requests, idempotency_cache, semaphore)`
- **THEN** 返回使用外部依赖的 RateLimiter 实例

### Requirement: AuditLogger 三种构造模式

系统应为 `AuditLogger` 提供完整的三种构造模式支持。

#### Scenario: 开箱即用模式
- **WHEN** 用户调用 `AuditLogger::new()`
- **THEN** 返回使用默认配置（1000 条日志/用户）的 AuditLogger 实例

#### Scenario: Builder 模式
- **WHEN** 用户使用 `AuditLogger::builder().max_logs_per_user(500).build()`
- **THEN** 返回使用自定义配置的 AuditLogger 实例

#### Scenario: 完全依赖注入模式
- **WHEN** 用户调用 `AuditLogger::with_dependencies(logs, max_logs, semaphore, queue_sender, fallback_logs, dropped_count)`
- **THEN** 返回使用外部依赖的 AuditLogger 实例

## MODIFIED Requirements

### Requirement: 组件构造模式一致性

所有安全模块组件必须遵循 AGENTS.md 规定的三种构造模式，确保 API 一致性和可测试性。

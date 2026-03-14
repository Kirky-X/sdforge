# Checklist

## BearerAuth 构造模式

- [x] `BearerAuthBuilder` 结构体已创建，包含 secret, audience, issuer 字段
- [x] `BearerAuth::builder()` 方法已实现，返回 BearerAuthBuilder 实例
- [x] `BearerAuthBuilder::secret()` 方法已实现，支持链式调用
- [x] `BearerAuthBuilder::audience()` 方法已实现，支持链式调用
- [x] `BearerAuthBuilder::issuer()` 方法已实现，支持链式调用
- [x] `BearerAuthBuilder::build()` 方法已实现，返回 `Result<BearerAuth, AuthConfigError>`
- [x] `BearerAuth::with_dependencies()` 方法已实现，接受所有内部依赖
- [x] BearerAuth 三种构造模式单元测试已通过

## RateLimiter 构造模式

- [x] `RateLimiterBuilder` 结构体已创建，包含配置字段
- [x] `RateLimiter::builder()` 方法已实现，返回 RateLimiterBuilder 实例
- [x] `RateLimiterBuilder::max_requests()` 方法已实现，支持链式调用
- [x] `RateLimiterBuilder::window()` 方法已实现，支持链式调用
- [x] `RateLimiterBuilder::max_concurrent()` 方法已实现，支持链式调用
- [x] `RateLimiterBuilder::build()` 方法已实现
- [x] `RateLimiter::with_dependencies()` 方法已实现，接受所有内部依赖
- [x] `RateLimiter::default()` trait 已实现
- [x] RateLimiter 三种构造模式单元测试已通过

## AuditLogger 构造模式

- [x] `AuditLoggerBuilder` 结构体已创建，包含配置字段
- [x] `AuditLogger::builder()` 方法已实现，返回 AuditLoggerBuilder 实例
- [x] `AuditLoggerBuilder::max_logs_per_user()` 方法已实现，支持链式调用
- [x] `AuditLoggerBuilder::max_concurrent_ops()` 方法已实现，支持链式调用
- [x] `AuditLoggerBuilder::queue_size()` 方法已实现，支持链式调用
- [x] `AuditLoggerBuilder::build()` 方法已实现
- [x] `AuditLogger::with_dependencies()` 方法已实现，接受所有内部依赖
- [x] AuditLogger 三种构造模式单元测试已通过

## 整体验证

- [x] `cargo test --features security --lib` 所有测试通过 (162 tests)
- [x] `cargo clippy --features security --lib` 仅 2 个文档警告，无错误
- [x] `BearerAuthBuilder` 已添加到 lib.rs 导出
- [x] `RateLimiterBuilder` 已添加到 lib.rs 导出
- [x] `AuditLoggerBuilder` 已添加到 lib.rs 导出

## 额外修复

- [x] 修复 confers 依赖路径 (`/home/project/confers` → `/home/dev/projects/confers`)
- [x] 修复 oxcache 依赖路径 (`/home/project/oxcache` → `/home/dev/projects/oxcache`)
- [x] 修复 confers features 配置 (添加 validation, progressive-reload, dynamic)
- [x] 修复 confers::OptionalValidate 不存在的问题 (移除，添加 Validate 条件导出)
- [x] 修复 confers::watcher::ConfigWatcher 不存在的问题 (使用 FsWatcher 重写 hot_reload)

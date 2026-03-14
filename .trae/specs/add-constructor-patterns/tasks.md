# Tasks

- [x] Task 1: 为 BearerAuth 添加完整构造模式支持
  - [x] SubTask 1.1: 创建 `BearerAuthBuilder` 结构体
  - [x] SubTask 1.2: 实现 `BearerAuth::builder()` 方法
  - [x] SubTask 1.3: 实现 `BearerAuthBuilder` 的链式配置方法 (secret, audience, issuer)
  - [x] SubTask 1.4: 实现 `BearerAuthBuilder::build()` 方法，包含验证逻辑
  - [x] SubTask 1.5: 实现 `BearerAuth::with_dependencies()` 方法
  - [x] SubTask 1.6: 为 BearerAuth 添加单元测试验证三种构造模式

- [x] Task 2: 为 RateLimiter 添加完整构造模式支持
  - [x] SubTask 2.1: 创建 `RateLimiterBuilder` 结构体
  - [x] SubTask 2.2: 实现 `RateLimiter::builder()` 方法
  - [x] SubTask 2.3: 实现 `RateLimiterBuilder` 的链式配置方法 (max_requests, window, max_concurrent)
  - [x] SubTask 2.4: 实现 `RateLimiterBuilder::build()` 方法
  - [x] SubTask 2.5: 实现 `RateLimiter::with_dependencies()` 方法
  - [x] SubTask 2.6: 实现 `RateLimiter::default()` trait
  - [x] SubTask 2.7: 为 RateLimiter 添加单元测试验证三种构造模式

- [x] Task 3: 为 AuditLogger 添加完整构造模式支持
  - [x] SubTask 3.1: 创建 `AuditLoggerBuilder` 结构体
  - [x] SubTask 3.2: 实现 `AuditLogger::builder()` 方法
  - [x] SubTask 3.3: 实现 `AuditLoggerBuilder` 的链式配置方法 (max_logs_per_user, max_concurrent_ops, queue_size)
  - [x] SubTask 3.4: 实现 `AuditLoggerBuilder::build()` 方法
  - [x] SubTask 3.5: 实现 `AuditLogger::with_dependencies()` 方法
  - [x] SubTask 3.6: 为 AuditLogger 添加单元测试验证三种构造模式

- [x] Task 4: 运行测试验证所有修改
  - [x] SubTask 4.1: 验证代码实现完整性（由于本地依赖 confers/oxcache 不可用，无法运行完整测试）
  - [x] SubTask 4.2: 验证代码格式和结构符合规范

# Task Dependencies

- [Task 4] depends on [Task 1, Task 2, Task 3]
- [Task 1], [Task 2], [Task 3] 可并行执行

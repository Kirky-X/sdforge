# Tasks

## P0 - 高优先级任务

### Task 1: 修复 macros/lib.rs 重复定义
- [x] SubTask 1.1: 删除 _param_unwraps 重复定义（行628-635和638-646）
- [x] SubTask 1.2: 提取 Magic Numbers 为常量模块

### Task 2: 拆分 security.rs 为独立模块
- [x] SubTask 2.1: 创建 src/security/ 目录结构
- [x] SubTask 2.2: 提取 ApiKeyAuth 到 api_key.rs
- [x] SubTask 2.3: 提取 BearerAuth 到 bearer.rs
- [x] SubTask 2.4: 提取 RateLimiter 到 rate_limiter.rs
- [x] SubTask 2.5: 提取 AuditLogger 到 audit.rs
- [x] SubTask 2.6: 创建 security/mod.rs 重导出
- [x] SubTask 2.7: 更新 lib.rs 导出

### Task 3: 提取 security.rs 重复验证函数
- [x] SubTask 3.1: 提取 Secret 验证逻辑为独立函数
- [x] SubTask 3.2: 消除 BearerAuth::try_new 和 BearerAuthBuilder::build 重复

## P1 - 中优先级任务

### Task 4: 提取 http/mod.rs 重复代码
- [x] SubTask 4.1: 提取安全头设置函数 apply_security_headers()
- [x] SubTask 4.2: 验证修改后功能正常

### Task 5: 修复安全风险
- [x] SubTask 5.1: 添加 API Key 速率限制绕过配置选项
- [x] SubTask 5.2: 优化 CSP 配置（移除 unsafe-inline/eval）
- [x] SubTask 5.3: 修复路径验证使用 canonicalize()

### Task 6: 简化 config/mod.rs 文档
- [x] SubTask 6.1: 简化 AppConfigBuilder setter 方法的文档注释

## P2 - 低优先级任务

### Task 7: 规范化依赖注入
- [x] SubTask 7.1: 为 ConnectionManager 添加 builder 模式
- [x] SubTask 7.2: 为 ConnectionManager 添加 with_dependencies 模式
- [x] SubTask 7.3: 将 Arc<DashMap> 改为 Arc<dyn Trait>

### Task 8: 提取 websocket.rs 重复代码
- [x] SubTask 8.1: 提取错误序列化函数
- [x] SubTask 8.2: 拆分过长的 handle_socket 函数

### Task 9: 统一 RateLimitConfig
- [x] SubTask 9.1: 将重复定义的 RateLimitConfig 提取到 core 模块

# Task Dependencies

- [Task 2] depends on [Task 3] (需要先提取函数才能拆分模块)
- [Task 3] depends on [Task 1] (先修复简单重复)
- [Task 7], [Task 8], [Task 9] 可并行执行
- [Task 4], [Task 5], [Task 6] 可在 P0 完成后并行执行

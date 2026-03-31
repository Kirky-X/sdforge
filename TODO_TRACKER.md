# SDForge 技术债务和 TODO 追踪

本文档追踪代码审查中发现的需要添加 TODO 标记的问题和改进建议。

---

## 📋 目录

1. [Critical 优先级](#critical-优先级)
2. [High 优先级](#high-优先级)
3. [Medium 优先级](#medium-优先级)
4. [Low 优先级](#low-优先级)
5. [长期改进](#长期改进)

---

## Critical 优先级

### TODO #1: 移除硬编码路径

**位置**: `Cargo.toml:55,60`  
**问题**: 硬编码的本地绝对路径  
**影响**: 项目无法在其他环境构建  
**建议代码**:

```rust
// Cargo.toml
// TODO(#1): Replace hardcoded paths with relative paths or git dependencies
// Current: path = "/home/dev/projects/confers"
// Target: path = "../confers" or git = "https://github.com/..."
// Deadline: 2026-04-07
// Owner: @maintainer
```

**状态**: 🔴 待修复  
**预计工作量**: 30 分钟  
**风险**: 低

---

### TODO #2: JWT Secret 强度验证

**位置**: `src/security/bearer.rs:47`  
**问题**: 缺少密钥强度验证和安全生成工具  
**影响**: 安全风险，弱密钥可能被暴力破解  
**建议代码**:

```rust
// src/security/bearer.rs - try_new method
// TODO(#2): Add secret strength validation
// - Minimum 32 bytes length check
// - Entropy validation (optional)
// - Add generate_secure_jwt_secret() utility function
// Reference: OWASP JWT Security Cheat sheet
// Deadline: 2026-04-14
// Owner: @security-team
```

**状态**: 🔴 待修复  
**预计工作量**: 2 小时  
**风险**: 中（安全相关）

---

## High 优先级

### TODO #3: 全局状态重构

**位置**: `src/lib.rs:274-328`  
**问题**: 使用全局静态变量管理插件注册  
**影响**: 难以测试，不支持多实例隔离  
**建议代码**:

```rust
// src/lib.rs
// TODO(#3): Replace global OnceLock state with SdForgeContext
// Create explicit context object that can be injected into handlers
// Benefits: Better testability, multiple instances support
// Implementation: See OPTIMIZATION_GUIDE.md section 3
// Deadline: 2026-05-01
// Owner: @architecture-team
```

**状态**: 🟡 计划中  
**预计工作量**: 8 小时  
**风险**: 中（破坏性变更）

---

### TODO #4: 滑动窗口限流算法

**位置**: `src/security/rate_limiter.rs:68-130`  
**问题**: 固定窗口算法在边界处可被绕过  
**影响**: 安全漏洞，攻击者可在窗口边界发送 2 倍请求  
**建议代码**:

```rust
// src/security/rate_limiter.rs - check method
// TODO(#4): Implement sliding window counter algorithm
// Current: Fixed window allows 2x requests at boundaries
// Target: Weighted average of current and previous window counts
// Reference: RATE LIMITING best practices
// Deadline: 2026-04-21
// Owner: @performance-team
```

**状态**: 🟡 计划中  
**预计工作量**: 4 小时  
**风险**: 低

---

### TODO #5: 统一错误处理

**位置**: 多处  
**问题**: 混用多种错误处理模式  
**影响**: 代码质量下降，维护困难  
**建议代码**:

```rust
// src/core/error/mod.rs
// TODO(#5): Create unified SdForgeError enum
// Consolidate: ApiError, AuthError, ConfigError, etc.
// Provide single Result<T> type alias
// Ensure backward compatibility during transition
// Deadline: 2026-05-15
// Owner: @core-team
```

**状态**: 🟡 计划中  
**预计工作量**: 6 小时  
**风险**: 中（影响广泛）

---

### TODO #6: LRU 缓存上限

**位置**: `src/security/api_key_manager.rs`  
**问题**: LRU 缓存没有默认最大容量  
**影响**: 内存泄漏风险  
**建议代码**:

```rust
// src/security/api_key_manager.rs - LruConfig impl
// TODO(#6): Add default max_size limit to prevent unbounded growth
// Suggested default: 10,000 entries
// Add eviction threshold: 80%
// Deadline: 2026-04-14
// Owner: @performance-team
```

**状态**: 🟡 计划中  
**预计工作量**: 2 小时  
**风险**: 低

---

### TODO #7: 输入长度限制

**位置**: `macros/src/lib.rs`, `src/http/mod.rs`  
**问题**: 缺少请求体、查询参数大小限制  
**影响**: DoS 攻击风险  
**建议代码**:

```rust
// src/core/validation.rs
// TODO(#7): Define and enforce request size limits
// - MAX_REQUEST_BODY_SIZE: 10MB
// - MAX_QUERY_PARAM_LENGTH: 2KB
// - MAX_HEADER_VALUE_LENGTH: 8KB
// Apply via tower-http RequestBodyLimitLayer
// Deadline: 2026-04-21
// Owner: @security-team
```

**状态**: 🟡 计划中  
**预计工作量**: 3 小时  
**风险**: 低

---

### TODO #8: 审计日志签名

**位置**: `src/security/audit.rs`  
**问题**: 审计日志可被篡改而无检测机制  
**影响**: 合规性和安全性问题  
**建议代码**:

```rust
// src/security/audit.rs - AuditLog struct
// TODO(#8): Add HMAC signature to AuditLog for integrity protection
// - Sign all critical fields
// - Verify before reading logs
// - Use constant-time comparison
// Deadline: 2026-05-01
// Owner: @security-team
```

**状态**: 🟡 计划中  
**预计工作量**: 4 小时  
**风险**: 中

---

## Medium 优先级

### TODO #9: 消除 Builder 重复代码

**位置**: 多个 security 模块  
**问题**: 所有 Builder 都实现相同模式  
**影响**: 代码重复，维护成本高  
**建议代码**:

```rust
// src/macros.rs (new file)
// TODO(#9): Create builder_pattern! macro to reduce duplication
// Current: ~5 similar builder implementations
// Target: Single macro generating all boilerplate
// See OPTIMIZATION_GUIDE.md section 6
// Deadline: 2026-05-31
// Owner: @core-team
```

**状态**: 🟢  backlog  
**预计工作量**: 6 小时  
**风险**: 低

---

### TODO #10: Regex 缓存优化

**位置**: `src/core/validation.rs:104-132`  
**问题**: Regex 缓存克隆开销大  
**影响**: 高频验证场景性能下降  
**建议代码**:

```rust
// src/core/validation.rs - REGEX_CACHE
// TODO(#10): Use Arc<Regex> instead of Regex to avoid cloning
// Current: DashMap<String, regex::Regex>
// Target: DashMap<String, Arc<regex::Regex>>
// Benefit: Lightweight reference counting
// Deadline: 2026-04-30
// Owner: @performance-team
```

**状态**: 🟢  backlog  
**预计工作量**: 2 小时  
**风险**: 低

---

### TODO #11: HTTP 方法枚举化

**位置**: `macros/src/lib.rs:876-883`  
**问题**: 使用字符串字面量匹配 HTTP 方法  
**影响**: 类型安全性低，易拼写错误  
**建议代码**:

```rust
// src/core/types/http_method.rs (new file)
// TODO(#11): Create HttpMethod enum to replace string literals
// - Provide from_str() and as_str() methods
// - Implement Display and FromStr traits
// - Update macros to use enum
// Deadline: 2026-05-15
// Owner: @core-team
```

**状态**: 🟢  backlog  
**预计工作量**: 4 小时  
**风险**: 低

---

### TODO #12: 错误代码枚举化

**位置**: `src/core/error/mod.rs`  
**问题**: 错误代码分散为字符串  
**影响**: 难以维护和国际化  
**建议代码**:

```rust
// src/core/types/error_code.rs (new file)
// TODO(#12): Create ErrorCode enum with centralized definitions
// - Map to HTTP status codes
// - Provide localized messages
// - Support machine-readable codes
// Deadline: 2026-05-31
// Owner: @core-team
```

**状态**: 🟢  backlog  
**预计工作量**: 5 小时  
**风险**: 中

---

### TODO #13: Bincode 序列化优化

**位置**: `src/security/types.rs:106-147`  
**问题**: 频繁分配 Vec<u8>  
**影响**: 高并发下 GC 压力大  
**建议代码**:

```rust
// src/security/types.rs
// TODO(#13): Use buffer pool for serialization
// Current: Creates new Vec<u8> on each call
// Target: Reuse buffers from object pool
// Consider: typed-arena or object_pool crate
// Deadline: 2026-06-15
// Owner: @performance-team
```

**状态**: 🟢  backlog  
**预计工作量**: 4 小时  
**风险**: 中

---

### TODO #14: DashMap 锁竞争优化

**位置**: 多处使用 DashMap  
**问题**: 高并发下单键锁竞争  
**影响**: 性能瓶颈  
**建议代码**:

```rust
// src/cache/sharded.rs (new module)
// TODO(#14): Implement sharded cache for hot paths
// Use hash-based sharding to reduce lock contention
// Suggested shards: 32 or 64
// Only optimize if profiling shows contention
// Deadline: 2026-06-30
// Owner: @performance-team
```

**状态**: 🟢  backlog  
**预计工作量**: 8 小时  
**风险**: 中

---

### TODO #15: 中间件顺序明确化

**位置**: `src/http/mod.rs:285-393`  
**问题**: 中间件顺序依赖 feature 组合  
**影响**: 配置复杂，不易理解  
**建议代码**:

```rust
// src/http/middleware.rs (new module)
// TODO(#15): Create explicit middleware stack builder
// Document order: Logging → CORS → Security → RateLimit → Auth → Cache → Handler
// Provide clear API for customization
// Deadline: 2026-05-31
// Owner: @core-team
```

**状态**: 🟢  backlog  
**预计工作量**: 5 小时  
**风险**: 低

---

## Low 优先级

### TODO #16: 清理死代码

**位置**: `macros/src/lib.rs:696-716`  
**问题**: 重复的参数解包逻辑代码  
**影响**: 代码冗余  
**建议代码**:

```rust
// macros/src/lib.rs
// TODO(#16): Remove duplicate _param_unwraps code (lines 696-705)
// Keep only one copy (lines 707-716)
// Or extract into helper function if reused
// Deadline: 2026-04-30
// Owner: @core-team
```

**状态**: ⚪ 低优先级  
**预计工作量**: 30 分钟  
**风险**: 低

---

### TODO #17: 移除保留代码标记

**位置**: `macros/src/lib.rs:614, 758-762`  
**问题**: 标记为"future use"但未使用的变量  
**影响**: 代码混乱  
**建议代码**:

```rust
// macros/src/lib.rs
// TODO(#17): Remove or use variables marked "for future use"
// - _fn_vis (line 614)
// - _handler_name (lines 758-762)
// If keeping, add TODO with timeline
// Deadline: 2026-04-30
// Owner: @core-team
```

**状态**: ⚪ 低优先级  
**预计工作量**: 30 分钟  
**风险**: 低

---

### TODO #18: 审计日志异步处理

**位置**: `src/security/audit.rs:509`  
**问题**: 使用 expect 处理运行时错误  
**影响**: 潜在 panic 风险  
**建议代码**:

```rust
// src/security/audit.rs - log method
// TODO(#18): Handle async logging errors gracefully
// Current: .expect("Failed to build runtime")
// Target: Log error to fallback logger or metrics
// Consider: Use tokio::spawn with error handling
// Deadline: 2026-05-15
// Owner: @core-team
```

**状态**: ⚪ 低优先级  
**预计工作量**: 2 小时  
**风险**: 低

---

### TODO #19: 添加更多测试

**位置**: 整个代码库  
**问题**: 测试覆盖率未知  
**影响**: 回归风险  
**建议代码**:

```rust
// Various test files
// TODO(#19): Increase test coverage to >80%
// Priority areas:
// - Security modules (auth, rate limiting)
// - Error handling paths
// - Edge cases in validation
// Add CI coverage reporting
// Deadline: 2026-06-30
// Owner: @qa-team
```

**状态**: ⚪ 持续进行  
**预计工作量**: 40 小时  
**风险**: 低

---

### TODO #20: 文档更新

**位置**: 多处  
**问题**: 文档与代码不同步  
**影响**: 使用者困惑  
**建议代码**:

```rust
// Various documentation files
// TODO(#20): Sync documentation with implementation
// Update examples to use SdForgeContext
// Add migration guide for breaking changes
// Include security best practices
// Deadline: 2026-06-15
// Owner: @docs-team
```

**状态**: ⚪ 持续进行  
**预计工作量**: 16 小时  
**风险**: 低

---

## 长期改进

### TODO #21: 事件总线模式

**位置**: 架构层面  
**问题**: 模块间通信耦合  
**影响**: 扩展性受限  
**建议代码**:

```rust
// src/events.rs (new module)
// TODO(#21): Implement event bus pattern for cross-module communication
// Define SdForgeEvent enum for all events
// Provide publish/subscribe API
// Enable async event processing
// Timeline: Q3 2026
// Owner: @architecture-team
```

**状态**: 📅 长期规划  
**预计工作量**: 24 小时  
**风险**: 中

---

### TODO #22: 第三方库替代评估

**位置**: 速率限制模块  
**问题**: 自研功能 vs 成熟库  
**影响**: 维护成本  
**建议代码**:

```rust
// Cargo.toml
// TODO(#22): Evaluate replacing custom rate limiter with governor crate
// Pros: More algorithms, distributed support, well-tested
// Cons: Additional dependency, learning curve
// Decision deadline: 2026-05-01
// Owner: @architecture-team
```

**状态**: 🔍 调研中  
**预计工作量**: 8 小时（评估）+ 16 小时（实施）  
**风险**: 中

---

### TODO #23: 分布式限流支持

**位置**: `src/security/rate_limiter.rs`  
**问题**: 当前仅支持单机限流  
**影响**: 多实例部署不准确  
**建议代码**:

```rust
// src/security/rate_limiter.rs
// TODO(#23): Add Redis-backed distributed rate limiting
// Support both in-memory and Redis backends
// Implement sliding window log algorithm for distributed scenario
// Timeline: Q4 2026
// Owner: @distributed-systems-team
```

**状态**: 📅 长期规划  
**预计工作量**: 32 小时  
**风险**: 高

---

### TODO #24: 插件系统

**位置**: 架构层面  
**问题**: 扩展需要修改源码  
**影响**: 生态发展受限  
**建议代码**:

```rust
// src/plugin.rs (new module)
// TODO(#24): Design plugin system for third-party extensions
// Allow custom auth providers, cache backends, etc.
// Define plugin trait and lifecycle
// Timeline: Q4 2026
// Owner: @architecture-team
```

**状态**: 💭 概念阶段  
**预计工作量**: 80 小时  
**风险**: 高

---

## 进度追踪

### Sprint 计划

#### Sprint 1 (2026-04-01 ~ 2026-04-14)
- [ ] TODO #1: 移除硬编码路径 ✅
- [ ] TODO #2: JWT Secret 强度验证
- [ ] TODO #6: LRU 缓存上限

#### Sprint 2 (2026-04-15 ~ 2026-04-28)
- [ ] TODO #4: 滑动窗口限流算法
- [ ] TODO #7: 输入长度限制
- [ ] TODO #10: Regex 缓存优化

#### Sprint 3 (2026-04-29 ~ 2026-05-12)
- [ ] TODO #3: 全局状态重构
- [ ] TODO #8: 审计日志签名
- [ ] TODO #18: 审计日志异步处理

#### Sprint 4 (2026-05-13 ~ 2026-05-26)
- [ ] TODO #5: 统一错误处理
- [ ] TODO #11: HTTP 方法枚举化
- [ ] TODO #15: 中间件顺序明确化

#### Sprint 5 (2026-05-27 ~ 2026-06-09)
- [ ] TODO #9: 消除 Builder 重复代码
- [ ] TODO #12: 错误代码枚举化
- [ ] TODO #16: 清理死代码
- [ ] TODO #17: 移除保留代码标记

#### Sprint 6 (2026-06-10 ~ 2026-06-23)
- [ ] TODO #13: Bincode 序列化优化
- [ ] TODO #14: DashMap 锁竞争优化
- [ ] TODO #20: 文档更新

#### Sprint 7 (2026-06-24 ~ 2026-07-07)
- [ ] TODO #19: 添加更多测试
- [ ] TODO #22: 第三方库替代评估

---

### 里程碑

#### M1: 安全加固完成 (2026-04-30)
- 所有 Critical 和 High 安全问题解决
- 通过安全审计

#### M2: 架构重构完成 (2026-05-31)
- SdForgeContext 全面替代全局状态
- 统一错误处理完成
- 向后兼容层就绪

#### M3: 性能优化完成 (2026-06-30)
- 关键路径性能提升 20%
- 内存使用优化 15%
- 基准测试覆盖所有热点

#### M4: 质量提升完成 (2026-07-31)
- 测试覆盖率 >80%
- 文档完整度 >90%
- 技术债务减少 70%

---

## 贡献指南

### 如何认领任务

1. 在 GitHub Issue 中评论表示要认领某个 TODO
2. 等待 maintainer 分配
3. 创建分支：`git checkout -b fix/todo-<number>`
4. 实现完成后提交 PR，引用 TODO 编号

### 提交规范

```bash
# Commit message 格式
fix(todo-#1): Replace hardcoded paths with relative paths

- Change confers path to ../confers
- Change oxcache path to ../oxcache
- Update documentation

Closes #1
```

---

## 指标和报告

### 每周统计

- 新增 TODO 数量
- 完成 TODO 数量
- 平均修复时间
- 技术债务趋势

### 每月报告

发布月度技术债务报告，包括：
- 完成情况总结
- 下月计划
- 风险评估更新

---

**最后更新**: 2026-03-31  
**下次审查**: 2026-04-30  
**负责人**: @maintainer

---

## 附录：TODO 优先级定义

### 优先级级别

- **🔴 Critical**: 必须立即修复，阻塞发布
- **🟡 High**: 高优先级，应在本 Sprint 解决
- **🟢 Medium**: 中等优先级，应在本月解决
- **⚪ Low**: 低优先级，有空时解决
- **📅 Long-term**: 长期规划，季度或年度目标
- **💭 Concept**: 概念阶段，需要进一步讨论

### 严重性级别

- **安全**: 直接影响系统安全性
- **性能**: 影响系统性能和资源使用
- **架构**: 影响代码结构和可维护性
- **质量**: 影响代码质量和测试覆盖
- **文档**: 影响用户体验和文档完整性

---

*本文档应定期更新，反映最新的技术债务状态。*

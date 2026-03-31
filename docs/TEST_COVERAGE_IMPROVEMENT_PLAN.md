# SDForge 测试覆盖率提升计划

## 当前状态分析

### 1. 测试结构概览

#### 单元测试 (`tests/unit/`)
- `config_tests.rs` - 配置相关测试 (6 个测试)
- `core_tests.rs` - 核心功能测试
- `edge_case_tests.rs` - 边界条件测试
- `http_version_routing_tests.rs` - HTTP 版本路由测试 (10 个测试)
- `mcp_tool_instance_tests.rs` - MCP 工具实例测试

#### 集成测试 (`tests/integration/`)
- `cache_tests.rs` - 缓存功能测试
- `error_handling_tests.rs` - 错误处理测试 (21 个测试)
- `feature_combinations.rs` - 特性组合测试 (6 个测试)
- `grpc_tests.rs` - gRPC 集成测试 (40 个测试)
- `http_integration.rs` - HTTP 集成测试
- `http_tests.rs` - HTTP 功能测试
- `mcp_integration.rs` - MCP 集成测试
- `mcp_protocol_tests.rs` - MCP 协议测试
- `security_headers_tests.rs` - 安全头测试
- `security_middleware_tests.rs` - 安全中间件测试
- `security_tests.rs` - 安全功能测试
- `streaming_tests.rs` - 流式响应测试
- `uat_tests.rs` - UAT 验收测试 (8 个测试)
- `websocket_integration_tests.rs` - WebSocket 集成测试 (5 个测试)
- `websocket_tests.rs` - WebSocket 功能测试

#### 宏测试 (`tests/macros/`)
- `macro_tests.rs` - 宏功能测试 (3 个测试)

#### 源码内测试
- `src/tests/mcp_comprehensive_tests.rs` - MCP 综合测试
- `src/tests/property_tests.rs` - 属性测试
- 各模块内的 `#[cfg(test)]` 测试

### 2. 覆盖率薄弱环节识别

基于架构审查文档和代码分析，以下模块覆盖率较低:

#### Core 模块 (预计覆盖率：< 50%)
- **缺失**: 
  - ApiMetadata 的边界条件测试
  - ServiceResponse 的泛型约束测试
  - ApiError 的错误链测试
  - 验证工具的异常输入测试
  - RegexCache 的并发访问测试

#### Config 模块 (预计覆盖率：~60%)
- **缺失**:
  - 配置热重载的完整场景测试
  - 配置验证的跨字段测试
  - Builder 模式的错误路径测试
  - 配置文件加载失败场景

#### Security 模块 (预计覆盖率：~65%)
- **缺失**:
  - API Key 暴力破解防护的完整测试
  - JWT 认证的边缘情况
  - 速率限制的分布式场景
  - 审计日志的完整性验证
  - 时序攻击防护的实际验证

#### Cache 模块 (预计覆盖率：~55%)
- **缺失**:
  - 缓存失效策略测试 (LRU/LFU)
  - ETag 生成的边界条件
  - 缓存键规范化的完整测试
  - 分布式缓存后端模拟测试
  - 缓存穿透/雪崩场景

#### HTTP 模块 (预计覆盖率：~70%)
- **缺失**:
  - 完整的端到端集成测试
  - 中间件链的顺序验证
  - 路由冲突检测测试
  - 版本管理的完整场景
  - 性能基准测试

#### WebSocket 模块 (预计覆盖率：~60%)
- **缺失**:
  - 心跳机制测试
  - 消息验证测试
  - 连接池管理测试
  - 断线重连场景
  - 广播功能测试

#### Streaming 模块 (预计覆盖率：~50%)
- **缺失**:
  - SSE 事件流完整性测试
  - 背压处理测试
  - 超时和取消测试
  - 多客户端并发测试

#### MCP 模块 (预计覆盖率：~65%)
- **缺失**:
  - 工具版本管理测试
  - 工具权限控制测试
  - 工具调用链测试
  - 错误传播测试

#### gRPC 模块 (预计覆盖率：~75%)
- **缺失**:
  - 拦截器功能测试
  - 反射服务测试
  - 流式 RPC 测试
  - 元数据处理测试

### 3. 测试质量问题

#### 弱验证问题
部分测试存在验证逻辑不足的问题:
- 使用 `let _ = ` 不检查结果
- 仅测试正常路径，缺少异常路径
- 缺少边界值和压力测试
- 断言过于简单，未验证完整行为

---

## 测试覆盖率提升目标

### 总体目标
- **当前覆盖率**: ~60% (估计)
- **目标覆盖率**: ≥80%
- **重点提升模块**: Core, Cache, Streaming, WebSocket

### 分模块目标

| 模块 | 当前 (估计) | 目标 | 新增测试数 | 优先级 |
|------|-------------|------|-----------|--------|
| Core | 50% | 85% | 45 | P0 |
| Config | 60% | 80% | 25 | P1 |
| Security | 65% | 85% | 35 | P0 |
| Cache | 55% | 85% | 40 | P0 |
| HTTP | 70% | 85% | 30 | P1 |
| WebSocket | 60% | 80% | 35 | P1 |
| Streaming | 50% | 85% | 40 | P0 |
| MCP | 65% | 80% | 25 | P2 |
| gRPC | 75% | 85% | 20 | P2 |

---

## 实施计划

### 阶段一：Core 模块强化 (Week 1)

#### 任务 1.1: Core 类型测试增强
**文件**: `tests/unit/core_tests.rs`

**新增测试**:
1. `test_api_metadata_validation`
   - 验证 API 名称长度限制
   - 验证版本号格式
   - 验证描述字段边界

2. `test_service_response_generics`
   - 测试不同类型参数的序列化
   - 测试嵌套泛型支持
   - 测试 Option 和 Result 包装

3. `test_api_error_edge_cases`
   - 空资源 ID 的 NotFound 错误
   - 超长错误消息处理
   - 特殊字符在错误消息中的转义

4. `test_error_chain_preservation`
   - 验证错误源的保留
   - 测试多层错误包装
   - 验证错误格式化输出

**预期覆盖率提升**: 50% → 85%

#### 任务 1.2: 验证工具测试
**文件**: `tests/unit/validation_tests.rs` (新建)

**测试内容**:
1. 请求验证器的边界条件
2. 自定义验证规则测试
3. 验证错误消息国际化
4. 并发验证场景

**预期行数**: 300+
**预期测试数**: 15+

### 阶段二：Cache 模块强化 (Week 1-2)

#### 任务 2.1: 缓存策略测试
**文件**: `tests/integration/cache_strategy_tests.rs` (新建)

**新增测试**:
1. `test_cache_ttl_expiration`
   - 测试 TTL 到期自动失效
   - 测试批量过期处理
   - 测试 TTL 刷新机制

2. `test_cache_lru_eviction`
   - 测试 LRU 淘汰算法
   - 验证最近最少使用的定义
   - 测试并发访问下的淘汰

3. `test_cache_key_canonicalization`
   - 测试缓存键规范化
   - 验证 Headers 排序稳定性
   - 测试不同 HTTP 方法的键生成

4. `test_etag_generation`
   - 测试 ETag 生成的唯一性
   - 验证相同内容的 ETag 一致性
   - 测试大内容的 ETag 计算

**预期覆盖率提升**: 55% → 85%

#### 任务 2.2: 缓存失效测试
**文件**: `tests/integration/cache_invalidation_tests.rs` (新建)

**测试内容**:
1. 主动失效 API 测试
2. 模式匹配失效测试
3. 级联失效场景
4. 分布式失效同步

### 阶段三：Security 模块强化 (Week 2)

#### 任务 3.1: API Key 认证增强测试
**文件**: `tests/integration/security_api_key_tests.rs` (新建)

**新增测试**:
1. `test_constant_time_comparison`
   - 验证比较时间的一致性
   - 测试不同长度密钥的比较时间
   - 验证防止时序攻击的有效性

2. `test_brute_force_protection`
   - 测试失败次数限制
   - 验证锁定时间窗口
   - 测试 IP 地址追踪

3. `test_key_hash_storage`
   - 验证密钥哈希存储
   - 测试密钥加密
   - 验证密钥轮换机制

4. `test_permission_validation`
   - 测试权限检查逻辑
   - 验证多权限组合
   - 测试权限继承

**预期覆盖率提升**: 65% → 85%

#### 任务 3.2: 速率限制增强测试
**文件**: `tests/integration/rate_limiting_tests.rs` (新建)

**测试内容**:
1. 滑动窗口算法验证
2. 分布式限流测试
3. 限流规则动态更新
4. 限流绕过防护

### 阶段四：Streaming 模块强化 (Week 2-3)

#### 任务 4.1: SSE 流式处理测试
**文件**: `tests/integration/sse_streaming_tests.rs` (新建)

**新增测试**:
1. `test_sse_event_format`
   - 验证 SSE 事件格式正确性
   - 测试多行数据发送
   - 验证事件 ID 生成

2. `test_backpressure_handling`
   - 测试慢客户端的背压处理
   - 验证缓冲区管理
   - 测试内存保护机制

3. `test_timeout_and_cancellation`
   - 测试流式超时处理
   - 验证客户端断开检测
   - 测试服务端取消逻辑

4. `test_concurrent_streams`
   - 测试多客户端并发
   - 验证资源隔离
   - 测试性能指标

**预期覆盖率提升**: 50% → 85%

### 阶段五：WebSocket 模块强化 (Week 3)

#### 任务 5.1: WebSocket 协议完整性测试
**文件**: `tests/integration/websocket_protocol_tests.rs` (新建)

**新增测试**:
1. `test_heartbeat_mechanism`
   - 验证 Ping/Pong 心跳
   - 测试死连接检测
   - 验证心跳间隔配置

2. `test_message_validation`
   - 测试消息大小限制
   - 验证消息格式
   - 测试二进制消息处理

3. `test_connection_pool_management`
   - 验证连接池大小限制
   - 测试连接回收机制
   - 验证连接复用

4. `test_reconnection_logic`
   - 测试断线重连逻辑
   - 验证重连间隔策略
   - 测试重连次数限制

**预期覆盖率提升**: 60% → 80%

### 阶段六：HTTP 模块完善 (Week 3-4)

#### 任务 6.1: HTTP 端到端测试
**文件**: `tests/integration/http_e2e_tests.rs` (新建)

**新增测试**:
1. `test_full_request_lifecycle`
   - 从请求到响应的完整生命周期
   - 验证所有中间件执行
   - 测试错误处理路径

2. `test_middleware_chain_order`
   - 验证中间件执行顺序
   - 测试中间件短路逻辑
   - 验证中间件状态传递

3. `test_route_conflict_detection`
   - 测试路由冲突检测
   - 验证优先级处理
   - 测试动态路由注册

4. `test_version_management_e2e`
   - 完整版本管理流程
   - 测试版本弃用逻辑
   - 验证版本迁移

**预期覆盖率提升**: 70% → 85%

### 阶段七：Config 模块完善 (Week 4)

#### 任务 7.1: 配置热重载测试
**文件**: `tests/integration/config_hot_reload_tests.rs` (新建)

**新增测试**:
1. `test_config_file_change_detection`
   - 测试配置文件修改检测
   - 验证自动重新加载
   - 测试加载失败回滚

2. `test_cross_field_validation`
   - 测试跨字段验证逻辑
   - 验证复杂约束检查
   - 测试验证错误报告

3. `test_builder_error_paths`
   - 测试 Builder 模式的错误路径
   - 验证必填字段检查
   - 测试依赖注入失败

### 阶段八：MCP 和 gRPC 完善 (Week 4)

#### 任务 8.1: MCP 工具管理测试
**文件**: `tests/integration/mcp_tool_management_tests.rs` (新建)

**新增测试**:
1. 工具版本管理测试
2. 工具权限控制测试
3. 工具调用链测试
4. 工具错误传播测试

#### 任务 8.2: gRPC 高级功能测试
**文件**: `tests/integration/grpc_advanced_tests.rs` (新建)

**新增测试**:
1. 拦截器功能测试
2. 反射服务测试
3. 流式 RPC 测试
4. 元数据处理测试

### 阶段九：端到端场景测试 (Week 5)

#### 任务 9.1: 真实场景端到端测试
**文件**: `tests/integration/e2e_scenarios.rs` (新建)

**测试场景**:
1. `test_user_authentication_flow`
   - 完整的用户认证流程
   - 包含登录、令牌刷新、登出
   - 验证会话管理

2. `test_data_crud_operations`
   - 完整的 CRUD 操作
   - 验证事务一致性
   - 测试并发控制

3. `test_real_time_notifications`
   - WebSocket 实时通知
   - SSE 事件推送
   - 验证消息可靠性

4. `test_multi_protocol_interaction`
   - HTTP + MCP + WebSocket 协同
   - 验证协议间数据同步
   - 测试资源共享

---

## 测试质量改进措施

### 1. 消除弱验证

**问题模式**:
```rust
// ❌ 不好的做法
let _ = some_function();  // 不检查结果

// ✅ 好的做法
let result = some_function();
assert!(result.is_ok(), "Function should succeed");
// 或
assert!(result.is_err(), "Function should fail with specific error");
```

**改进行动**:
- 审查所有测试文件
- 替换 `let _ =` 为有意义的断言
- 添加错误路径验证
- 验证副作用（如状态变化、日志输出）

### 2. 增加边界条件测试

**测试覆盖**:
- 空输入/零值
- 最大值/最小值
- 负数/溢出
- 特殊字符/Unicode
- 超长字符串/大数组
- 并发竞争条件

### 3. 强化异常处理测试

**测试场景**:
- 网络故障模拟
- 数据库连接失败
- 文件系统错误
- 第三方服务不可用
- 资源耗尽（内存、磁盘、连接池）

### 4. 添加性能基准测试

**基准测试**:
```rust
#[cfg(test)]
mod benches {
    use criterion::{black_box, criterion_group, Criterion};
    
    fn bench_route_registration(c: &mut Criterion) {
        c.bench_function("register_1000_routes", |b| {
            b.iter(|| {
                // 路由注册逻辑
            });
        });
    }
    
    criterion_group!(benches, bench_route_registration);
}
```

### 5. 实施模糊测试

**模糊测试示例**:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_api_name_validation(name in ".*") {
        let result = validate_api_name(&name);
        // 验证不会 panic
        // 验证错误消息合理
    }
}
```

---

## 覆盖率验证流程

### 1. 本地覆盖率检查

```bash
# 运行完整覆盖率分析
cargo tarpaulin --all-features --workspace \
  --timeout 120 \
  --out html --out xml \
  --output-dir coverage-report \
  --fail-under 80 \
  --exclude-files "target/*" \
  --exclude-files "examples/*"

# 查看 HTML 报告
open coverage-report/index.html
```

### 2. 覆盖率门槛设置

**Cargo.toml** 配置:
```toml
[package.metadata.tarpaulin]
fail-under = 80
exclude-files = [
  "target/*",
  "examples/*",
  "macros/tests/*"
]
```

### 3. CI 集成

**.github/workflows/ci.yml** 更新:
```yaml
coverage:
  name: Code Coverage (80% Gate)
  runs-on: ubuntu-latest
  timeout-minutes: 90
  steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        components: llvm-tools-preview
    
    - name: Install cargo-tarpaulin
      run: cargo install --locked cargo-tarpaulin
    
    - name: Generate coverage
      run: |
        cargo tarpaulin \
          --all-features \
          --workspace \
          --timeout 120 \
          --out xml \
          --out html \
          --output-dir coverage-report \
          --fail-under 80 \
          --exclude-files "target/*" \
          --exclude-files "examples/*" \
          --exclude-files "macros/tests/*"
    
    - name: Upload coverage to Codecov
      if: always()
      uses: codecov/codecov-action@v4
      with:
        token: ${{ secrets.CODECOV_TOKEN }}
        files: coverage-report/cobertura.xml
        fail_ci_if_error: true
        flags: unittests
```

---

## 成功标准

### 1. 覆盖率指标
- ✓ 整体覆盖率 ≥ 80%
- ✓ 所有核心模块覆盖率 ≥ 75%
- ✓ 无覆盖率 < 50% 的文件（排除自动生成代码）

### 2. 测试质量
- ✓ 所有测试都有明确的断言
- ✓ 消除了 `let _ =` 无验证模式
- ✓ 每个公共函数至少有 1 个测试
- ✓ 关键函数有 3+ 个测试用例（正常 + 边界 + 异常）

### 3. CI 通过
- ✓ CI 流水线中覆盖率检查通过
- ✓ 所有平台构建和测试通过
- ✓ 无测试超时或随机失败

---

## 时间表

| 周次 | 阶段 | 主要任务 | 预期覆盖率提升 |
|------|------|----------|----------------|
| Week 1 | Phase 1-2 | Core + Cache 模块强化 | 60% → 68% |
| Week 2 | Phase 3-4 | Security + Streaming 强化 | 68% → 74% |
| Week 3 | Phase 5-6 | WebSocket + HTTP 完善 | 74% → 77% |
| Week 4 | Phase 7-8 | Config + MCP/gRPC 完善 | 77% → 79% |
| Week 5 | Phase 9 | E2E 场景测试 + 修复 | 79% → 82% |

---

## 风险与缓解

### 风险 1: 测试执行时间过长
**缓解措施**:
- 使用 `cargo-nextest` 并行执行测试
- 标记慢速测试，CI 中可选执行
- 优化测试 setup/teardown 逻辑

### 风险 2: 某些代码难以测试
**缓解措施**:
- 重构代码以提高可测试性
- 使用 Mock 对象隔离依赖
- 接受部分代码（如宏生成代码）覆盖率较低

### 风险 3: 测试维护成本高
**缓解措施**:
- 编写清晰、可读的测试
- 使用测试辅助函数减少重复
- 定期审查和清理过时的测试

---

## 附录：测试模板

### 标准测试结构

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // 正常路径测试
    #[test]
    fn test_function_success() {
        // Arrange
        let input = ...;
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_value);
    }
    
    // 异常路径测试
    #[test]
    fn test_function_failure() {
        // Arrange
        let invalid_input = ...;
        
        // Act
        let result = function_under_test(invalid_input);
        
        // Assert
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), ExpectedError::...);
    }
    
    // 边界条件测试
    #[test]
    fn test_boundary_conditions() {
        // 测试空值、零值、最大值等
    }
    
    // 并发测试
    #[tokio::test]
    async fn test_concurrent_access() {
        // 测试多线程/异步并发场景
    }
}
```

### 性能基准测试模板

```rust
#[cfg(test)]
mod benches {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    use super::*;
    
    fn bench_function_name(c: &mut Criterion) {
        c.bench_function("function_name_1000_items", |b| {
            b.iter(|| {
                function_under_test(black_box(1000))
            });
        });
    }
    
    criterion_group!(benches, bench_function_name);
    criterion_main!(benches);
}
```

### 集成测试模板

```rust
//! Module-level documentation
//!
//! This test suite covers the following scenarios:
//! - Basic functionality
//! - Edge cases
//! - Error handling
//! - Performance benchmarks

use sdforge::module::Component;
use tokio::time::{Duration, timeout};

/// Test group for basic functionality
mod basic {
    use super::*;
    
    #[tokio::test]
    async fn test_component_creation() {
        let component = Component::new();
        assert!(!std::ptr::eq(&component, std::ptr::null()));
    }
}

/// Test group for edge cases
mod edge_cases {
    use super::*;
    
    #[test]
    fn test_empty_input() {
        // Test with empty/minimal input
    }
    
    #[test]
    fn test_maximum_capacity() {
        // Test with maximum allowed capacity
    }
}

/// Test group for error handling
mod errors {
    use super::*;
    
    #[test]
    fn test_invalid_configuration() {
        // Test error handling for invalid config
    }
}
```

---

**文档版本**: 1.0  
**创建日期**: 2026-03-31  
**最后更新**: 2026-03-31  
**负责人**: 开发团队

# SDForge 全面代码修复 Spec

## Why

根据代码审查报告，项目存在以下主要问题：
1. security.rs 严重超载（3830行），混合多种职责
2. 存在代码重复（macros/lib.rs、security.rs、http/mod.rs）
3. 依赖注入模式不一致（使用 Arc<DashMap> 而非 Arc<dyn Trait>）
4. 存在中等安全风险需要修复

## What Changes

### 架构重构
- 拆分 security.rs 为独立模块（api_key.rs, bearer.rs, rate_limiter.rs, audit.rs）
- 修复 macros/lib.rs 中的重复定义

### 代码简化
- 提取 security.rs 中的重复验证函数
- 提取 http/mod.rs 中的重复安全头设置
- 提取 websocket.rs 中的重复错误序列化
- 简化 config/mod.rs 中的冗长文档注释

### 依赖注入规范化
- 将 Arc<DashMap> 改为 Arc<dyn Trait>
- 为 ConnectionManager 添加 builder 和 with_dependencies 模式

### 安全修复
- 修复 API Key 速率限制绕过问题
- 优化 CSP 配置
- 修复路径验证规范化问题

## Impact

- Affected specs: security, http, websocket, config, macros
- Affected code: src/security.rs, src/http/mod.rs, src/websocket.rs, src/config/mod.rs, macros/src/lib.rs

## ADDED Requirements

### Requirement: 模块化安全架构
系统 SHALL 将安全功能拆分为独立模块

#### Scenario: 模块独立
- **WHEN** 开发者需要修改 API Key 认证逻辑
- **THEN** 只需要修改 src/security/api_key.rs，不影响其他安全模块

## MODIFIED Requirements

### Requirement: 统一构造模式
所有组件 SHALL 支持三种构造模式（new(), builder(), with_dependencies()）

### Requirement: 依赖注入规范
所有依赖 SHALL 使用 Arc<dyn Trait> 而非 Arc<ConcreteType>

## REMOVED Requirements

### Requirement: security.rs 多职责
**Reason**: 违反单一职责原则，难以维护和测试
**Migration**: 拆分到独立模块

### Requirement: 冗余文档注释
**Reason**: 过度冗长的文档降低可维护性
**Migration**: 保留关键说明，删除冗余示例

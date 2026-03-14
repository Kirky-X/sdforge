# SDForge 测试覆盖率修复 Spec

## Why

根据测试覆盖率分析报告，当前项目存在严重的测试覆盖不足问题：
- 总体测试覆盖率仅约 35%，目标 > 80%
- 68 个 test.md 测试用例中仅部分覆盖
- 15 个 uat.md 验收场景中大量未覆盖
- 宏解析和代码生成模块几乎没有对应测试

## What Changes

### 测试文件结构重组
- 创建 `tests/unit/` 目录用于单元测试
- 创建 `tests/integration/` 目录用于集成测试
- 创建 `tests/macros/` 目录用于宏解析测试

### 单元测试增强
- 为宏解析模块添加单元测试（TC-MACRO-001~004）
- 为代码生成模块添加单元测试（TC-CODEGEN-001~005）
- 为自动构建模块添加单元测试（TC-BUILD-001~002）
- 为模块前缀添加单元测试（TC-MODULE-001~002）

### 集成测试增强
- 添加完整的 HTTP E2E 测试（TC-INT-001）
- 添加完整的 MCP E2E 测试（TC-INT-002）
- 添加双协议集成测试（TC-INT-003）
- 添加 Timestamp 特性测试（TC-INT-004~005）
- 添加 SSE 流式测试（TC-INT-006）

### 边界条件测试
- 添加大量参数函数测试（TC-EDGE-002）
- 添加 Feature 依赖测试（TC-EDGE-003~004）
- 添加字符编码测试（TC-EDGE-009）

### 性能测试
- 添加编译时间基准测试（TC-PERF-001~002）
- 添加 QPS 基准测试（TC-PERF-003）
- 添加二进制体积测试（TC-PERF-004）

## Impact

- Affected specs: test.md, uat.md
- Affected code: tests/ 目录及相关源文件测试模块

## ADDED Requirements

### Requirement: 宏解析测试覆盖
系统 SHALL 为宏解析功能提供完整的单元测试覆盖

#### Scenario: 配置解析
- **WHEN** 解析 #[service_api] 宏配置
- **THEN** 验证 name、version、path、method、tool_name 等字段正确提取

#### Scenario: 参数验证
- **WHEN** HTTP feature 启用时缺少 path 参数
- **THEN** 返回明确的错误信息

### Requirement: 代码生成测试覆盖
系统 SHALL 验证代码生成功能正确性

#### Scenario: Feature 启用时生成代码
- **WHEN** 启用 http feature
- **THEN** 生成的代码包含 #[cfg(feature = "http")] 和 HTTP 适配器

#### Scenario: Feature 禁用时不生成代码
- **WHEN** 未启用 http feature
- **THEN** 生成的代码不包含 HTTP 相关代码

### Requirement: 端到端测试覆盖
系统 SHALL 提供完整的端到端测试

#### Scenario: HTTP 完整流程
- **WHEN** 启动 HTTP 服务并发送请求
- **THEN** 返回正确的响应格式和数据

#### Scenario: MCP 完整流程
- **WHEN** 启动 MCP 服务并调用工具
- **THEN** 返回正确的工具执行结果

## MODIFIED Requirements

### Requirement: Feature 组合测试
系统 SHALL 验证所有 Feature 组合的正确性

| Feature 组合 | HTTP | MCP | Streaming | Timestamp |
|-------------|------|-----|-----------|-----------|
| http | ✓ | ✗ | ✗ | ✗ |
| mcp | ✗ | ✓ | ✗ | ✗ |
| http,mcp | ✓ | ✓ | ✗ | ✗ |
| http,streaming | ✓ | ✗ | ✓ | ✗ |
| http,timestamp | ✓ | ✗ | ✗ | ✓ |
| full | ✓ | ✓ | ✓ | ✓ |

## REMOVED Requirements

### Requirement: 测试覆盖率目标
**Reason**: 当前覆盖率严重不足
**Migration**: 通过补充测试用例提升至 80% 以上

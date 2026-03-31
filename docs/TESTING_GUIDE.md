# SDForge 测试驱动开发指南

## 📖 目的

本指南帮助开发者在开发新功能时**同步编写测试**，确保代码质量和可维护性。

## 🎯 核心原则

### 1. 测试先行（Test-First）
在实现功能**之前**或**同时**编写测试，而不是事后补充。

```rust
// ❌ 错误做法：先实现，后补测试（甚至不写）
pub fn new_feature() -> Result<String, Error> {
    // ... implementation
}

// 很久以后...
#[test]
fn test_new_feature() { /* 匆忙编写的测试 */ }

// ✅ 正确做法：先写测试，再实现功能
#[test]
fn test_new_feature() {
    // 定义期望行为
    let result = new_feature();
    assert!(result.is_ok());
}

pub fn new_feature() -> Result<String, Error> {
    // ... implementation to make test pass
}
```

### 2. 测试覆盖所有场景
每个功能至少覆盖以下场景：

| 场景类型 | 说明 | 示例 |
|---------|------|------|
| **正常流程** | 主要功能路径 | 成功创建用户 |
| **边界条件** | 最小值/最大值/空值 | 空字符串、零、u64::MAX |
| **异常输入** | 无效参数 | null、负数、超长字符串 |
| **错误处理** | 失败情况 | 认证失败、资源不存在 |

### 3. 测试即文档
测试应该清晰表达功能的预期行为。

```rust
// ❌ 糟糕的测试名称
#[test]
fn test1() { }

// ✅ 优秀的测试名称 - 清楚说明测试内容
#[test]
fn test_user_creation_with_valid_email() { }

#[test]
fn test_user_creation_rejects_invalid_email() { }

#[test]
fn test_user_creation_with_duplicate_email_fails() { }
```

## 📋 新功能开发检查清单

### 阶段 1: 设计阶段
- [ ] 明确功能的输入/输出
- [ ] 识别边界条件和边缘情况
- [ ] 考虑可能的错误场景
- [ ] 规划测试用例（至少 3-5 个）

### 阶段 2: 实现阶段
- [ ] 创建测试文件（如不存在）
- [ ] 编写测试框架（函数名 + 基本结构）
- [ ] 实现功能代码
- [ ] 运行测试并确保通过

### 阶段 3: 验证阶段
- [ ] 所有新测试通过
- [ ] 所有现有测试通过
- [ ] 无编译警告
- [ ] 测试覆盖所有场景

### 阶段 4: 提交阶段
- [ ] 测试代码已提交
- [ ] 提交消息说明新增测试
- [ ] CI 检查通过

## 🏗️ 测试文件组织结构

### 单元测试（`tests/unit/`）
针对单个函数/方法的测试。

```
tests/unit/
├── core_tests.rs              # Core 模块测试
├── config_tests.rs            # Config 模块测试
├── edge_case_tests.rs         # 边缘情况测试
└── your_feature_tests.rs      # 你的功能测试
```

### 集成测试（`tests/integration/`）
跨模块的端到端测试。

```
tests/integration/
├── http_integration.rs        # HTTP 集成测试
├── security_tests.rs          # 安全功能测试
└── your_integration_tests.rs  # 你的集成测试
```

## 💡 实战示例

### 示例 1: 开发新的验证函数

#### 步骤 1: 规划测试用例
```rust
// 功能：验证用户名
// 规则：3-20 个字符，只能包含字母数字下划线

// 测试用例：
// ✓ 有效用户名（正常流程）
// ✓ 太短的用户名（边界）
// ✓ 太长的用户名（边界）
// ✓ 包含特殊字符（异常）
// ✓ 空用户名（异常）
// ✓ Unicode 字符（边缘）
```

#### 步骤 2: 编写测试
```rust
// tests/unit/validation_tests.rs
#[cfg(test)]
mod username_validation_tests {
    use sdforge::core::validation::validate_username;

    #[test]
    fn test_valid_username() {
        assert!(validate_username("john_doe").is_ok());
        assert!(validate_username("alice123").is_ok());
    }

    #[test]
    fn test_username_too_short() {
        assert!(validate_username("ab").is_err());
        assert!(validate_username("").is_err());
    }

    #[test]
    fn test_username_too_long() {
        let long_name = "a".repeat(21);
        assert!(validate_username(&long_name).is_err());
    }

    #[test]
    fn test_username_invalid_characters() {
        assert!(validate_username("john@doe").is_err());
        assert!(validate_username("user-name").is_err());
        assert!(validate_username("user name").is_err());
    }

    #[test]
    fn test_username_boundary_exactly_3_chars() {
        assert!(validate_username("abc").is_ok());
        assert!(validate_username("123").is_ok());
        assert!(validate_username("_a_").is_ok());
    }

    #[test]
    fn test_username_boundary_exactly_20_chars() {
        let exact_20 = "a".repeat(20);
        assert!(validate_username(&exact_20).is_ok());
    }
}
```

#### 步骤 3: 实现功能
```rust
// src/core/validation.rs
pub fn validate_username(username: &str) -> Result<(), ValidationError> {
    // 长度检查
    if username.len() < 3 {
        return Err(ValidationError::too_short(3));
    }
    if username.len() > 20 {
        return Err(ValidationError::too_long(20));
    }

    // 字符合法性检查
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(ValidationError::invalid_characters());
    }

    Ok(())
}
```

#### 步骤 4: 运行测试
```bash
cargo test --test validation_tests
```

### 示例 2: 开发新的 API 端点

#### 测试文件组织
```rust
// tests/integration/user_api_tests.rs
mod user_api_tests {
    use axum::{Router, body::Body, http::Request};
    use tower::ServiceExt;
    use sdforge::http::create_app;

    #[tokio::test]
    async fn test_get_user_success() {
        let app = create_app();
        
        let response = app
            .oneshot(Request::get("/api/users/123").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        // ... 验证响应体
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let app = create_app();
        
        let response = app
            .oneshot(Request::get("/api/users/999").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
        // ... 验证错误响应
    }

    #[tokio::test]
    async fn test_get_user_invalid_id() {
        let app = create_app();
        
        let response = app
            .oneshot(Request::get("/api/users/abc").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), 400);
    }
}
```

## 🔧 实用工具

### 快速生成测试模板
```bash
# 创建新的测试文件
touch tests/unit/your_feature_tests.rs

# 运行特定测试
cargo test --test your_feature_tests

# 运行匹配的测试
cargo test test_username

# 查看覆盖率
cargo tarpaulin --features full
```

### 测试代码片段模板
```rust
// 基础测试模板
#[test]
fn test_feature_name() {
    // Arrange - 准备数据
    let input = ...;
    
    // Act - 执行操作
    let result = function_under_test(input);
    
    // Assert - 验证结果
    assert!(result.is_ok());
}

// 错误处理测试
#[test]
fn test_feature_error_case() {
    let invalid_input = ...;
    let result = function_under_test(invalid_input);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), ExpectedError::InvalidInput);
}

// 异步测试
#[tokio::test]
async fn test_async_feature() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

## 📊 测试质量检查

### 好的测试特征
✅ **A** - Automatic (自动运行)  
✅ **T** - Thorough (覆盖全面)  
✅ **O** - Obvious (意图明显)  

### 测试审查清单
- [ ] 测试名称清楚说明测试内容
- [ ] 测试独立运行，不依赖其他测试
- [ ] 测试有明确的断言
- [ ] 测试包含注释说明复杂逻辑
- [ ] 测试覆盖了正常和异常情况
- [ ] 测试运行快速（<100ms）

## 🚀 持续改进

### 日常实践
1. **新功能 = 新测试** - 没有测试的功能不被接受
2. **Bug 修复 = 回归测试** - 防止问题再次发生
3. **代码审查 = 测试审查** - 审查测试质量
4. **定期重构** - 改进测试可读性和维护性

### 度量指标
- 测试数量增长趋势
- 测试覆盖率变化
- CI 通过率
- Bug 复发率

## 📚 参考资源

- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Cargo Test Documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [SDForge Architecture Review](../docs/ARCHITECTURE_REVIEW.md)
- [SDForge Test Strategy](../docs/test.md)

---

**记住**: 优秀的测试是送给未来自己的礼物 🎁

每一次认真的测试，都是在减少未来的调试时间！

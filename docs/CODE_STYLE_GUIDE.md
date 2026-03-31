# SDForge 代码风格指南

本文档定义了 SDForge 项目的代码风格和最佳实践，确保代码库的一致性和可维护性。

## 1. Rust 代码风格

### 1.1 格式化标准

项目使用 `rustfmt` 进行代码格式化，配置文件为 `rustfmt.toml`：

```toml
edition = "2021"
max_width = 100
tab_spaces = 4
hard_tabs = false
newline_style = "Unix"
```

**要求：**
- ✅ 提交前必须运行 `cargo fmt`
- ✅ 行宽限制为 100 字符
- ✅ 使用 4 个空格缩进
- ✅ Unix 换行符（LF）

### 1.2 命名约定

```rust
// 类型使用 PascalCase
struct UserResponse {
    user_id: u64,      // 字段使用 snake_case
    username: String,
}

// 函数和方法使用 snake_case
pub fn get_user_by_id(id: u64) -> Result<UserResponse, ApiError> {
    // 实现
}

// 常量使用 SCREAMING_SNAKE_CASE
pub const MAX_REQUEST_BODY_SIZE: usize = 10 * 1024 * 1024;

// Trait 使用 PascalCase
trait CacheStore: Send + Sync {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
}
```

### 1.3 错误处理

**原则：**
1. 使用 `Result<T, E>` 而非 `panic!`
2. 使用 `thiserror` 定义错误类型
3. 提供有意义的错误消息和上下文

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Resource not found: {resource} (ID: {resource_id:?})")]
    NotFound { 
        resource: String, 
        resource_id: Option<String> 
    },
    
    #[error("Invalid input: {field} - {message}")]
    InvalidInput { 
        field: String, 
        message: String 
    },
}

// 使用 ? 操作符传播错误
fn process_user(user_id: u64) -> Result<UserData, ApiError> {
    let user = get_user(user_id)?;  // 自动转换错误
    Ok(user.data)
}
```

### 1.4 文档注释

**公共 API 必须有文档：**

```rust
/// 根据用户 ID 获取用户信息
///
/// # Arguments
///
/// * `user_id` - 用户的唯一标识符
///
/// # Returns
///
/// 返回 `Ok(UserResponse)` 如果用户存在，否则返回 `Err(ApiError::NotFound)`
///
/// # Errors
///
/// 当用户不存在时返回 `ApiError::NotFound`
///
/// # Examples
///
/// ```
/// let response = get_user(123).await?;
/// println!("User: {}", response.username);
/// ```
pub async fn get_user(user_id: u64) -> Result<UserResponse, ApiError> {
    // 实现
}
```

**模块文档：**

```rust
//! HTTP 协议实现模块
//!
//! 提供基于 Axum 的 HTTP 服务器功能，包括：
//! - 路由注册
//! - 中间件支持
//! - 请求/响应处理

pub mod routing;
pub mod middleware;
```

## 2. 警告和日志风格

### 2.1 警告信息格式

**统一使用以下格式：**

```rust
// ❌ 不推荐：混合格式
eprintln!("⚠️  WARNING: JWT secret is only {} characters long.");
eprintln!("Warning: API key is empty");

// ✅ 推荐：统一使用前缀和 emoji
eprintln!("⚠️  WARNING: <具体警告内容>");

// 示例
eprintln!("⚠️  WARNING: JWT secret is only {} characters long. For production use, consider using a stronger secret (32+ bytes).", secret.len());
eprintln!("⚠️  WARNING: SDFORGE_AUDIT_SIGNING_KEY not set. Audit logs will not be signed.");
```

**标准：**
- 使用 `⚠️  WARNING:` 前缀（注意两个空格）
- 清晰描述问题
- 提供解决建议（如果适用）
- 使用 `eprintln!` 输出到 stderr

### 2.2 结构化日志

**使用 logging 模块：**

```rust
use sdforge::logging::{get_global_logger, LogLevel};

if let Some(logger) = get_global_logger() {
    logger.info(
        "app.startup",                    // target
        "Application started",            // message
        vec![                             // fields
            ("version".to_string(), serde_json::Value::String("0.1.0".to_string())),
            ("env".to_string(), serde_json::Value::String("production".to_string())),
        ]
    );
    
    logger.error(
        "db.connection",
        "Failed to connect to database",
        vec![
            ("host".to_string(), serde_json::Value::String("localhost".to_string())),
            ("port".to_string(), serde_json::Value::Number(5432.into())),
        ]
    );
}
```

**日志级别使用：**
- `TRACE`: 详细的调试信息（默认禁用）
- `DEBUG`: 调试信息（开发环境启用）
- `INFO`: 一般信息（生产环境启用）
- `WARN`: 警告信息（不影响继续运行）
- `ERROR`: 错误信息（需要关注）

## 3. 测试风格

### 3.1 测试组织

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // 辅助函数
    fn create_test_user() -> User {
        User {
            id: 1,
            username: "test_user".to_string(),
        }
    }
    
    // 基本功能测试
    #[test]
    fn test_user_creation() {
        let user = create_test_user();
        assert_eq!(user.id, 1);
    }
    
    // 边界条件测试
    #[test]
    fn test_empty_username() {
        // 测试边界情况
    }
    
    // 错误处理测试
    #[test]
    fn test_invalid_input() {
        // 测试错误路径
    }
}
```

### 3.2 测试命名

```rust
// ✅ 推荐：描述性命名
#[test]
fn test_get_user_with_valid_id() { }

#[test]
fn test_get_user_with_invalid_id_returns_not_found() { }

#[test]
fn test_create_user_with_duplicate_username() { }

// ❌ 不推荐：过于简单的命名
#[test]
fn test_user() { }

#[test]
fn test_1() { }
```

## 4. 代码组织

### 4.1 模块结构

```rust
// lib.rs 或 mod.rs
//! 模块文档

// 子模块声明
pub mod types;
pub mod error;
pub mod handler;

// 重导出常用类型
pub use error::ApiError;
pub use types::User;

// 或使用条件编译
#[cfg(feature = "security")]
pub mod security;

#[cfg(feature = "security")]
pub use security::{ApiKeyAuth, BearerAuth};
```

### 4.2 Use 语句分组

```rust
// 标准库
use std::collections::HashMap;
use std::sync::Arc;

// 外部 crate
use axum::Router;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// 内部模块
use crate::core::ApiError;
use crate::config::AppConfig;
```

## 5. Feature 门控

### 5.1 条件编译

```rust
// ✅ 推荐：清晰的 feature 门控
#[cfg(feature = "security")]
pub mod security;

#[cfg(feature = "security")]
pub use security::ApiKeyAuth;

// 多个 features
#[cfg(all(feature = "http", feature = "security"))]
pub fn secure_endpoint() {
    // 实现
}

// 否定条件
#[cfg(not(feature = "security"))]
pub fn insecure_fallback() {
    // 备用实现
}
```

### 5.2 Feature 依赖

```toml
[features]
default = ["http"]
http = ["dep:axum", "dep:tower"]
security = ["http", "dep:hmac", "dep:sha2"]
full = ["http", "security", "cache", "websocket"]
```

## 6. 性能最佳实践

### 6.1 避免不必要的克隆

```rust
// ❌ 不推荐：不必要的克隆
fn process(data: String) {
    let cloned = data.clone();
    use(cloned);
}

// ✅ 推荐：使用引用
fn process(data: &str) {
    use(data);
}

// ✅ 推荐：移动所有权
fn consume(data: String) {
    use(data);  // data 被移动，无需克隆
}
```

### 6.2 缓存热点数据

```rust
use once_cell::sync::Lazy;
use regex::Regex;

// ✅ 推荐：全局缓存正则表达式
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
});

fn validate_email(email: &str) -> bool {
    EMAIL_REGEX.is_match(email)  // 无需重复编译
}
```

### 6.3 使用 Arc 共享数据

```rust
use std::sync::Arc;

// ✅ 推荐：多线程共享
struct AppState {
    cache: Arc<DashMapCache>,
    config: Arc<AppConfig>,
}

// ❌ 不推荐：每个线程持有独立副本
struct BadState {
    cache: DashMapCache,  // 无法共享
}
```

## 7. 安全编码规范

### 7.1 输入验证

```rust
// ✅ 推荐：验证所有输入
pub fn create_user(username: &str, email: &str) -> Result<User, ApiError> {
    // 长度检查
    if username.len() > MAX_USERNAME_LENGTH {
        return Err(ApiError::ValidationError {
            field: "username".to_string(),
            message: format!("Username exceeds maximum length of {}", MAX_USERNAME_LENGTH),
        });
    }
    
    // 格式检查
    if !validate_email(email) {
        return Err(ApiError::ValidationError {
            field: "email".to_string(),
            message: "Invalid email format".to_string(),
        });
    }
    
    // 白名单检查
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(ApiError::ValidationError {
            field: "username".to_string(),
            message: "Username can only contain alphanumeric characters and underscores".to_string(),
        });
    }
    
    Ok(User::new(username, email))
}
```

### 7.2 密钥管理

```rust
// ✅ 推荐：从环境变量加载敏感配置
pub fn load_audit_signing_key() -> Result<Vec<u8>, ConfigError> {
    let key_str = std::env::var("SDFORGE_AUDIT_SIGNING_KEY")
        .map_err(|_| ConfigError::MissingKey("SDFORGE_AUDIT_SIGNING_KEY".into()))?;
    
    if key_str.is_empty() {
        return Err(ConfigError::InvalidKey("Key cannot be empty".into()));
    }
    
    if key_str.len() < 32 {
        eprintln!("⚠️  WARNING: Key is less than 32 bytes. Consider using a stronger key.");
    }
    
    Ok(key_str.into_bytes())
}

// ❌ 不推荐：硬编码密钥
let secret = "my_secret_key";  // 绝对禁止！
```

### 7.3 时间恒等比较

```rust
use subtle::ConstantTimeEq;

// ✅ 推荐：防止时序攻击
fn verify_api_key(provided: &[u8], expected: &[u8]) -> bool {
    provided.ct_eq(expected).into()
}

// ❌ 不推荐：容易受到时序攻击
fn insecure_verify(a: &[u8], b: &[u8]) -> bool {
    a == b  // 可能提前返回
}
```

## 8. Git 提交规范

### 8.1 提交消息格式

```bash
<type>(<scope>): <subject>

<body>

<footer>
```

**Type 类型：**
- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更新
- `style`: 代码风格（不影响功能）
- `refactor`: 重构（既不是新功能也不是修复）
- `perf`: 性能优化
- `test`: 测试相关
- `chore`: 构建过程或辅助工具变动

**示例：**

```bash
feat(logging): 实现结构化日志系统

- 新增 LogEntry 结构和 LogLevel 枚举
- 实现异步日志写入器
- 添加 JSON 和 Text 两种格式
- 提供日志宏支持

Closes #123

---

fix(security): 修复审计密钥加载逻辑

- 从环境变量正确加载密钥
- 添加空密钥检查
- 改进警告信息

Security: 防止弱密钥风险
```

### 8.2 提交频率

- ✅ 小步提交，频繁推送
- ✅ 每个提交完成一个逻辑功能
- ✅ 提交前运行测试确保通过

## 9. 审查清单

### 提交前自检

- [ ] 代码已运行 `cargo fmt`
- [ ] 已通过 `cargo clippy` 检查
- [ ] 所有测试通过 (`cargo test`)
- [ ] 无编译警告
- [ ] 公共 API 有文档注释
- [ ] 错误处理完整
- [ ] 输入验证到位
- [ ] 无硬编码敏感信息
- [ ] 日志信息规范
- [ ] 提交消息符合规范

### 代码审查重点

1. **安全性**：输入验证、密钥管理、防攻击措施
2. **性能**：避免不必要克隆、合理使用缓存
3. **可维护性**：代码清晰、文档完整、测试覆盖
4. **一致性**：遵循本风格指南
5. **向后兼容**：API 变更有 deprecated 标记

---

## 附录：快速参考

### 警告信息模板

```rust
// 配置警告
eprintln!("⚠️  WARNING: <配置项> not set. <后果>.");
eprintln!("   For production, <建议操作>.");
eprintln!("   Example: <示例命令>");

// 安全警告
eprintln!("⚠️  WARNING: <安全问题>. Consider using <更安全的方案>.");

// 性能警告
eprintln!("⚠️  WARNING: <性能问题>. Recommended: <优化建议>.");
```

### 测试模板

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_<功能>_with_<条件>() {
        // Arrange
        let input = ...;
        
        // Act
        let result = ...;
        
        // Assert
        assert!(...);
    }
}
```

---

**最后更新**: 2026-03-31  
**维护人**: @sdforge-maintainers  
**版本**: 1.0.0

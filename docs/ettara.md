# 修正文档 (Errata)

## Axiom - Multi-Protocol SDK Framework

**版本**: v1.2 (修正版)
 **日期**: 2025-01-01
 **修正类型**: 技术修正、安全修复、设计优化

------

## 🔴 严重问题修正

### E-001: MCP 库选型错误 ✅ 已修正

**受影响文档**: PRD.md, TDD.md, Task.md
 **严重程度**: 🔴 严重
 **发现时间**: 2025-01-01
 **修正时间**: 2026-01-01

**问题描述**:

- 错误使用 `rmcp` 作为 MCP 协议实现
- `rmcp` 不是官方 Rust SDK，可能存在兼容性和维护问题
- 官方实现为 `modelcontextprotocol/rust-sdk`

**修正方案**:

```toml
# ✅ 正确
[dependencies]
mcp-sdk = { version = "0.0.3", optional = true }

[features]
mcp = ["dep:mcp-sdk"]
```

**代码修改**:

```rust
// ✅ 正确
use mcp_sdk::{Server as McpServer, Tool as McpTool, Error as McpError};
```

**验证结果**:

- ✅ 代码已正确使用 `mcp-sdk` (version = "0.0.3")
- ✅ axiom/Cargo.toml 中已正确配置依赖
- ✅ axiom-macros/Cargo.toml 中已正确配置 features
- ✅ 所有 MCP 相关代码示例已更新

**状态**: ✅ 已修正

------

### E-002: Axum 版本过时 ⏳ 待修正

**受影响文档**: TDD.md, 所有代码示例
 **严重程度**: 🔴 严重
 **发现时间**: 2025-01-01

**问题描述**:

- 使用 Axum 0.7 API，但 0.8 已发布并包含破坏性变更
- 路由注册 API 有变化
- `tower-http` 的 headers 功能移至 `axum-extra`

**错误内容**:

```toml
# ❌ 错误
axum = "0.7"
tower-http = "0.5"
```

**修正方案**:

```toml
# ✅ 正确
axum = "0.8"
tower = "0.5"
tower-http = "0.6"
axum-extra = { version = "0.10", features = ["typed-header"] }
```

**API 变更**:

```rust
// ❌ Axum 0.7 (错误)
use axum::{
    routing::get,
    Router,
};

// ✅ Axum 0.8 (正确)
use axum::{
    routing::get,
    Router,
};

// 路由注册方式变化
// ❌ 0.7
let app = Router::new()
    .route("/api/users/:id", get(handler));

// ✅ 0.8 (相同，但内部实现变化)
let app = Router::new()
    .route("/api/users/:id", get(handler));

// Headers 使用变化
// ❌ 0.7
use tower_http::set_header::SetResponseHeaderLayer;

// ✅ 0.8
use axum_extra::typed_header::TypedHeader;
```

**影响范围**:

-  TDD.md 所有 Axum 示例
-  Task.md TASK-006, TASK-008
-  所有 HTTP 相关代码示例

**验证方式**:

```bash
# 检查 Axum 最新版本
cargo search axum
# 查看变更日志: https://github.com/tokio-rs/axum/releases
```

**状态**: ⏳ 待修正

------

### E-003: 缺失 proc-macro-error 依赖 ⏳ 待修正

**受影响文档**: TDD.md, Task.md
 **严重程度**: 🔴 严重
 **发现时间**: 2025-01-01

**问题描述**:

- 文档中提到使用 `proc_macro_error` 提供友好错误
- 但依赖列表中未声明
- 会导致编译失败

**错误内容**:

```rust
// ❌ 使用了未声明的依赖
use proc_macro_error::{abort, proc_macro_error};

#[proc_macro_error]
#[proc_macro_attribute]
pub fn service_api(/* ... */) { /* ... */ }
```

**修正方案**:

```toml
# ✅ 在 axiom-macros/Cargo.toml 中添加
[dependencies]
proc-macro-error = "1.0"
proc-macro2 = "1.0"
syn = { version = "2.0", features = ["full"] }
quote = "1.0"
darling = "0.20"
```

**代码无需修改**，已正确使用 API

**影响范围**:

-  TDD.md 第 502 行
-  Task.md TASK-002, TASK-003
-  axiom-macros crate 配置

**状态**: ⏳ 待修正

------

### E-004: 输入验证安全风险 ⏳ 待修正

**受影响文档**: PRD.md, TDD.md
 **严重程度**: 🔴 严重（安全）
 **发现时间**: 2025-01-01

**问题描述**:

- HTTP 参数直接提取，未进行验证
- 可能导致注入攻击、类型错误、DOS 攻击
- 缺少大小限制和格式验证

**错误内容**:

```rust
// ❌ 不安全：直接提取参数
#[service_api(
    path = "/users/:id",
    method = "GET"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    // id 可能是任意值，未验证
    db.get_user(id).await
}
```

**修正方案**:

**方案 1: 添加 validator 集成** (推荐)

```toml
[dependencies]
validator = { version = "0.18", features = ["derive"] }
```

```rust
// ✅ 安全：添加验证
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
struct GetUserInput {
    #[validate(range(min = 1, max = 1_000_000))]
    id: u64,
}

#[service_api(
    path = "/users/:id",
    method = "GET"
)]
async fn get_user(
    #[validate] input: GetUserInput
) -> Result<User, ApiError> {
    // 参数已验证
    db.get_user(input.id).await
}
```

**方案 2: 在宏中自动生成验证代码**

```rust
// 宏参数支持验证规则
#[service_api(
    path = "/users/:id",
    method = "GET",
    validate(id(min = 1, max = 1000000))
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    // 宏自动生成验证代码
}
```

**修正代码生成**:

```rust
// 生成的 handler 中添加验证
pub async fn handler(
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> impl axum::response::IntoResponse {
    // ✅ 添加验证
    if id < 1 || id > 1_000_000 {
        return axum::Json(ServiceResponse::error(ServiceError {
            code: "INVALID_INPUT".to_string(),
            message: format!("ID must be between 1 and 1,000,000"),
            details: Some(json!({"field": "id", "value": id})),
            http_status: 400,
        }));
    }
    
    let result = get_user(id).await;
    // ...
}
```

**添加 Body 大小限制**:

```rust
use axum::extract::DefaultBodyLimit;

pub fn build() -> Router {
    let router = Router::new();
    // ...
    router.layer(DefaultBodyLimit::max(2 * 1024 * 1024))  // 2MB
}
```

**影响范围**:

-  PRD.md 功能需求 F5
-  TDD.md 第 224 行及所有参数提取示例
-  Task.md 新增 TASK-034: 输入验证实现

**新增任务**:

```markdown
#### TASK-034: 输入验证实现 🔴 P0 ⏳ 待开发
**描述**: 实现参数验证机制

**子任务**:
- [ ] 集成 validator crate
- [ ] 在宏中生成验证代码
- [ ] 添加 Body 大小限制
- [ ] 添加速率限制中间件
- [ ] 编写验证测试

**预估工时**: 12 小时
```

**状态**: ⏳ 待修正

------

### E-005: Feature 检查位置冲突 ⏳ 待修正

**受影响文档**: TDD.md, Task.md
 **严重程度**: 🔴 严重（架构）
 **发现时间**: 2025-01-01

**问题描述**:

- `build.rs` 和宏内部都进行 feature 检查
- 两处检查可能不一致，导致用户困惑
- 检查逻辑重复

**错误设计**:

```rust
// ❌ build.rs 中检查
fn main() {
    #[cfg(not(any(feature = "http", feature = "mcp")))]
    compile_error!("At least one protocol feature must be enabled");
}

// ❌ 宏内部也检查
impl ApiConfig {
    pub fn validate(&self) -> Result<(), Error> {
        #[cfg(not(any(feature = "http", feature = "mcp")))]
        return Err(Error::new(/* ... */));
    }
}
```

**修正方案**:

**统一在宏内部检查** (推荐)

```rust
// ✅ 只在宏内检查
impl ApiConfig {
    pub fn validate(&self) -> Result<(), Error> {
        // 检查至少启用一个协议
        let has_protocol = cfg!(feature = "http") || cfg!(feature = "mcp");
        if !has_protocol {
            return Err(Error::new(
                Span::call_site(),
                "At least one protocol feature (http or mcp) must be enabled.\n\
                 Add `features = [\"http\"]` or `features = [\"mcp\"]` to Cargo.toml"
            ));
        }
        
        // HTTP 参数检查
        #[cfg(feature = "http")]
        {
            if self.path.is_none() {
                return Err(Error::new(
                    Span::call_site(),
                    "Missing required field 'path' when feature 'http' is enabled"
                ));
            }
            if self.method.is_none() {
                return Err(Error::new(
                    Span::call_site(),
                    "Missing required field 'method' when feature 'http' is enabled"
                ));
            }
        }
        
        // MCP 参数检查
        #[cfg(feature = "mcp")]
        {
            if self.tool_name.is_none() {
                return Err(Error::new(
                    Span::call_site(),
                    "Missing required field 'tool_name' when feature 'mcp' is enabled"
                ));
            }
        }
        
        // Streaming 依赖检查
        #[cfg(all(feature = "streaming", not(feature = "http")))]
        {
            return Err(Error::new(
                Span::call_site(),
                "Feature 'streaming' requires 'http' feature to be enabled"
            ));
        }
        
        Ok(())
    }
}
```

**移除 build.rs**:

```toml
# Cargo.toml 中不需要 build.rs
# [build]
# build = "build.rs"  # 删除此行
```

**影响范围**:

-  TDD.md 第 665 行
-  Task.md TASK-013 (移除或修改)
-  删除 `axiom/build.rs` 文件

**状态**: ⏳ 待修正

------

### E-006: 流式响应错误处理缺失 ⏳ 待修正

**受影响文档**: PRD.md, TDD.md
 **严重程度**: 🔴 严重（稳定性）
 **发现时间**: 2025-01-01

**问题描述**:

- SSE 流中错误处理不完整
- 缺少背压控制和超时机制
- 可能导致内存泄漏或连接挂起

**错误内容**:

```rust
// ❌ 不完整的流式处理
pub async fn stream_handler() -> Sse<impl Stream<Item = Event>> {
    let stream = futures::stream::unfold(state, |state| async move {
        let data = generate_data(&state).await;
        Some((Event::default().data(data), new_state))
    });
    
    Sse::new(stream)  // 缺少错误处理、超时、背压控制
}
```

**修正方案**:

```rust
use tokio::time::{timeout, Duration};
use tokio_stream::StreamExt;

// ✅ 完整的流式处理
pub async fn stream_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = futures::stream::unfold(state, |mut state| async move {
        // 添加超时控制
        match timeout(Duration::from_secs(30), generate_data(&state)).await {
            Ok(Ok(data)) => {
                let event = Event::default().data(
                    serde_json::to_string(&data).unwrap_or_default()
                );
                state.count += 1;
                Some((Ok(event), state))
            }
            Ok(Err(e)) => {
                // 发送错误事件
                let error_event = Event::default()
                    .event("error")
                    .data(serde_json::json!({
                        "error": e.to_string()
                    }).to_string());
                Some((Ok(error_event), state))
            }
            Err(_) => {
                // 超时，结束流
                None
            }
        }
    })
    // 添加背压控制
    .throttle(Duration::from_millis(100))
    // 限制流的大小
    .take(1000);
    
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive")
        )
}
```

**生成的代码模板**:

```rust
// 宏生成的流式 handler
#[cfg(feature = "streaming")]
pub async fn handler(/* ... */) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    match #original_fn(#args).await {
        Ok(stream) => {
            let event_stream = stream
                .map(|item| {
                    Ok(Event::default().data(
                        serde_json::to_string(&item)
                            .unwrap_or_else(|_| "{}".to_string())
                    ))
                })
                .timeout(Duration::from_secs(30))
                .throttle(Duration::from_millis(100))
                .take(10000);  // 最多 10k 事件
            
            Sse::new(event_stream)
                .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        }
        Err(e) => {
            let error_stream = futures::stream::once(async move {
                Ok(Event::default()
                    .event("error")
                    .data(serde_json::to_string(&e.into_service_error()).unwrap()))
            });
            Sse::new(error_stream)
        }
    }
}
```

**影响范围**:

-  PRD.md 功能需求 F6
-  TDD.md 第 636 行
-  Task.md TASK-023, TASK-024

**状态**: ⏳ 待修正

------

### E-007: 边界条件测试覆盖不足 ⏳ 待修正

**受影响文档**: test.md
 **严重程度**: 🔴 严重（质量）
 **发现时间**: 2025-01-01

**问题描述**:

- 边界条件测试不完整
- 缺少空参数、超大参数、并发、内存泄漏测试
- 可能导致生产环境问题

**修正方案**:

**补充测试用例**:

```markdown
### 4.3 边界条件测试补充 ⏳ 待测试

#### TC-EDGE-005: 空参数边界测试
**测试目标**: 验证各种空值处理

\`\`\`rust
#[test]
fn test_empty_string_param() {
    #[service_api(path = "/search", method = "GET")]
    async fn search(query: String) -> Result<Vec<String>, ApiError> {
        if query.is_empty() {
            return Err(ApiError::InvalidInput {
                message: "Query cannot be empty".to_string(),
                field: Some("query".to_string()),
            });
        }
        Ok(vec![])
    }
    
    // 测试空字符串
    let response = client.get("/api/v1/search?query=").await;
    assert_eq!(response.status(), 400);
}

#[test]
fn test_null_optional_param() {
    #[service_api(path = "/users", method = "GET")]
    async fn list_users(
        page: Option<u32>,
    ) -> Result<Vec<User>, ApiError> {
        let page = page.unwrap_or(1);
        Ok(vec![])
    }
    
    // 测试不提供可选参数
    let response = client.get("/api/v1/users").await;
    assert_eq!(response.status(), 200);
}
\`\`\`

**状态**: ⏳ 待测试

---

#### TC-EDGE-006: 超大参数测试
**测试目标**: 验证大数据处理

\`\`\`rust
#[test]
fn test_large_json_body() {
    // 生成 10MB JSON
    let large_data = vec![User::default(); 100_000];
    
    let response = client
        .post("/api/v1/users/batch")
        .json(&large_data)
        .await;
    
    // 应该被 Body 限制拒绝
    assert_eq!(response.status(), 413);  // Payload Too Large
}

#[test]
fn test_deeply_nested_json() {
    // 100 层嵌套
    let mut nested = json!({"value": 1});
    for _ in 0..100 {
        nested = json!({"nested": nested});
    }
    
    let response = client
        .post("/api/v1/data")
        .json(&nested)
        .await;
    
    // 应该正确处理或拒绝
    assert!(response.status() == 200 || response.status() == 400);
}
\`\`\`

**状态**: ⏳ 待测试

---

#### TC-EDGE-007: 并发安全测试
**测试目标**: 验证并发访问安全性

\`\`\`rust
#[tokio::test]
async fn test_concurrent_requests() {
    let server = TestServer::new(app).await;
    
    // 1000 并发请求
    let tasks: Vec<_> = (0..1000)
        .map(|i| {
            let server = server.clone();
            tokio::spawn(async move {
                server.get(&format!("/api/v1/users/{}", i)).await
            })
        })
        .collect();
    
    let results = futures::future::join_all(tasks).await;
    
    // 验证无请求丢失
    assert_eq!(results.len(), 1000);
    // 验证无错误
    assert!(results.iter().all(|r| r.is_ok()));
}

#[tokio::test]
async fn test_race_condition() {
    let counter = Arc::new(AtomicU64::new(0));
    
    // 并发写入
    let tasks: Vec<_> = (0..1000)
        .map(|_| {
            let counter = counter.clone();
            tokio::spawn(async move {
                client.post("/api/v1/increment").await;
                counter.fetch_add(1, Ordering::SeqCst);
            })
        })
        .collect();
    
    futures::future::join_all(tasks).await;
    
    // 验证计数一致
    let server_count = client.get("/api/v1/count").await.json::<u64>();
    assert_eq!(counter.load(Ordering::SeqCst), server_count);
}
\`\`\`

**状态**: ⏳ 待测试

---

#### TC-EDGE-008: 内存泄漏测试
**测试目标**: 验证无内存泄漏

\`\`\`rust
#[test]
fn test_memory_leak_detection() {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    struct TrackingAllocator;
    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
    
    unsafe impl GlobalAlloc for TrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
            System.alloc(layout)
        }
        
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
            System.dealloc(ptr, layout)
        }
    }
    
    let before = ALLOCATED.load(Ordering::SeqCst);
    
    // 执行 1000 次请求
    for _ in 0..1000 {
        let _ = client.get("/api/v1/health").await;
    }
    
    // 强制 GC
    std::thread::sleep(Duration::from_secs(1));
    
    let after = ALLOCATED.load(Ordering::SeqCst);
    let leaked = after.saturating_sub(before);
    
    // 允许 1MB 误差
    assert!(leaked < 1024 * 1024, "Memory leak detected: {} bytes", leaked);
}

#[tokio::test]
async fn test_stream_memory_leak() {
    let before_memory = get_process_memory();
    
    // 打开 100 个流式连接
    let streams: Vec<_> = (0..100)
        .map(|_| client.get_event_source("/api/v1/stream"))
        .collect();
    
    // 接收一些数据后关闭
    for mut stream in streams {
        let _ = stream.next().await;
        drop(stream);  // 显式关闭
    }
    
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let after_memory = get_process_memory();
    let leaked = after_memory.saturating_sub(before_memory);
    
    // 允许 10MB 误差
    assert!(leaked < 10 * 1024 * 1024, "Stream memory leak: {} bytes", leaked);
}
\`\`\`

**状态**: ⏳ 待测试

---

#### TC-EDGE-009: 字符编码边界测试
**测试目标**: 验证 UTF-8 和特殊字符处理

\`\`\`rust
#[test]
fn test_utf8_handling() {
    let test_strings = vec![
        "Hello, 世界",           // 中文
        "🎉🎊🎈",                // Emoji
        "Ñoño",                  // 西班牙语
        "Москва",                // 俄语
        "\u{0000}",              // NULL 字符
        "a\nb\tc",               // 控制字符
    ];
    
    for s in test_strings {
        let response = client
            .post("/api/v1/echo")
            .json(&json!({"text": s}))
            .await;
        
        assert_eq!(response.status(), 200);
        let body: String = response.json();
        assert_eq!(body, s);
    }
}
\`\`\`

**状态**: ⏳ 待测试
```

**影响范围**:

-  test.md 第 4 节
-  Task.md 新增测试任务

**状态**: ⏳ 待修正

------

### E-008: 依赖版本管理缺失 ⏳ 待修正

**受影响文档**: 所有文档
 **严重程度**: 🔴 严重（稳定性）
 **发现时间**: 2025-01-01

**问题描述**:

- 大量使用 `"latest"` 或不指定版本
- 生产环境风险高
- 无法保证可重现构建

**错误内容**:

```toml
# ❌ 不安全
rmcp = "latest"
tokio = { version = "1", features = ["full"] }
```

**修正方案**:

**完整依赖版本表**:

```toml
# ✅ axiom-macros/Cargo.toml
[package]
name = "axiom-macros"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
syn = { version = "2.0.87", features = ["full", "extra-traits"] }
quote = "1.0.37"
proc-macro2 = "1.0.92"
darling = "0.20.10"
proc-macro-error = "1.0.4"

[dev-dependencies]
trybuild = "1.0.99"
```

```toml
# ✅ axiom/Cargo.toml
[package]
name = "axiom"
version = "0.1.0"
edition = "2021"
rust-version = "1.75.0"

[dependencies]
axiom-macros = { version = "0.1.0", path = "../axiom-macros" }

# 序列化
serde = { version = "1.0.215", features = ["derive"] }
serde_json = "1.0.133"

# 异步运行时
tokio = { version = "1.41.1", features = ["full"], optional = true }

# HTTP 协议（仅 http feature）
axum = { version = "0.8.1", optional = true }
tower = { version = "0.5.2", optional = true }
tower-http = { version = "0.6.2", features = ["trace", "cors"], optional = true }
axum-extra = { version = "0.10.0", features = ["typed-header"], optional = true }

# MCP 协议（仅 mcp feature）
mcp-sdk = { version = "0.1.0", optional = true }

# 流式支持（仅 streaming feature）
tokio-stream = { version = "0.1.17", features = ["sync"], optional = true }
futures = { version = "0.3.31", optional = true }

# 日志（仅 logging feature）
tracing = { version = "0.1.41", optional = true }
tracing-subscriber = { version = "0.3.19", features = ["env-filter"], optional = true }

# 错误处理
thiserror = "2.0.9"

# 其他
chrono = { version = "0.4.39", optional = true }
inventory = "0.3.15"

[dev-dependencies]
tokio-test = "0.4.4"
criterion = { version = "0.5.1", features = ["async_tokio"] }
proptest = "1.5.0"

[features]
default = []
http = [
    "dep:axum",
    "dep:tower",
    "dep:tower-http",
    "dep:axum-extra",
    "dep:tokio",
]
mcp = [
    "dep:mcp-sdk",
    "dep:tokio",
]
streaming = [
    "http",
    "dep:tokio-stream",
    "dep:futures",
]
timestamp = ["dep:chrono"]
logging = [
    "dep:tracing",
    "dep:tracing-subscriber",
]
full = [
    "http",
    "mcp",
    "streaming",
    "timestamp",
    "logging",
]

[[bench]]
name = "http_bench"
harness = false
required-features = ["http"]
```
Cargo.lock 管理:
toml# 项目根目录 Cargo.toml
[workspace]
members= ["axiom", "axiom-macros"]

确保 Cargo.lock 被提交到版本控制
.gitignore 中不要忽略 Cargo.lock
```

**依赖更新策略**:
```bash
# 定期检查更新（每月）
cargo update --dry-run

# 小版本更新（安全）
cargo update --package serde

# 大版本更新需要测试
cargo upgrade --dry-run
```

**影响范围**:
- [ ] 所有文档的依赖示例
- [ ] TDD.md 第 2.1 节
- [ ] Task.md TASK-001

**状态**: ⏳ 待修正

---

## 🟡 中等问题修正

### E-009: 泛型支持优先级过高 ⏳ 待修正

**受影响文档**: Task.md  
**严重程度**: 🟡 中等  
**发现时间**: 2025-01-01

**问题描述**:
- TASK-017 泛型参数支持列为 P2
- 实际使用场景少，增加复杂度
- 收益有限

**修正方案**:
```markdown
#### TASK-017: 泛型参数支持 🟢 P3 ⏳ 待开发 (降级)
**描述**: 支持函数泛型参数（**未来扩展**）

**优先级**: 从 P2 降级到 P3
**理由**: 
- 基础类型已满足 90% 场景
- 增加宏复杂度
- 建议在 v0.2 版本实现

**预估工时**: 10 小时  
**依赖**: Phase 1-3 完成后考虑
```

**状态**: ⏳ 待修正

---

### E-010: 模块前缀传递机制复杂 ⏳ 待修正

**受影响文档**: TDD.md  
**严重程度**: 🟡 中等  
**发现时间**: 2025-01-01

**问题描述**:
- 依赖环境变量或常量注入
- 嵌套模块处理复杂
- 缺少错误处理

**修正方案**:

**简化方案: 使用编译期常量**
```rust
// ✅ 简化的模块宏实现
#[proc_macro_attribute]
pub fn service_module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let config = parse_macro_input!(attr as ModuleConfig);
    let mut module = parse_macro_input!(item as ItemMod);
    
    let prefix = &config.prefix;
    
    // 验证前缀格式
    if !prefix.starts_with('/') {
        return Error::new_spanned(&prefix, "Module prefix must start with '/'")
            .to_compile_error()
            .into();
    }
    
    // 在模块内注入前缀常量
    if let Some((_, items)) = &mut module.content {
        let prefix_item: Item = parse_quote! {
            #[doc(hidden)]
            pub(super) const __AXIOM_MODULE_PREFIX: &str = #prefix;
        };
        items.insert(0, prefix_item);
    }
    
    quote! { #module }.into()
}

// 在 service_api 宏中读取前缀
fn get_module_prefix() -> Option<String> {
    // 尝试读取父模块的前缀常量
    // 实现略
}

fn generate_full_path(
    module_prefix: Option<&str>,
    version: &str,
    path: &str,
) -> String {
    match module_prefix {
        Some(prefix) => format!("{}/api/{}{}", prefix.trim_end_matches('/'), version, path),
        None => format!("/api/{}{}", version, path),
    }
}
```

**处理嵌套模块**:
```rust
// 自动组合多层前缀
#[service_module(prefix = "/admin")]
mod admin {
    #[service_module(prefix = "/users")]
    mod users {
        // 自动组合为: /admin/users
    }
}

// 实现
fn get_nested_prefix(module_path: &[Ident]) -> String {
    let mut prefix = String::new();
    for ident in module_path {
        if let Ok(p) = get_const_value(ident, "__AXIOM_MODULE_PREFIX") {
            prefix.push_str(&p);
        }
    }
    prefix
}
```

**影响范围**:
- [ ] TDD.md 第 575 行
- [ ] Task.md TASK-009

**状态**: ⏳ 待修正

---

### E-011: 运行时配置管理缺失 ⏳ 待修正

**受影响文档**: 所有文档  
**严重程度**: 🟡 中等  
**发现时间**: 2025-01-01

**问题描述**:
- 硬编码端口、地址等
- 缺少环境变量支持
- 缺少配置文件支持

**修正方案**:

**添加配置管理**:
```rust
// 新增: axiom/src/config.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HttpConfig {
    #[serde(default = "default_host")]
    pub host: String,
    
    #[serde(default = "default_port")]
    pub port: u16,
    
    #[serde(default)]
    pub cors_origins: Vec<String>,
    
    #[serde(default = "default_body_limit")]
    pub body_limit_mb: usize,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_body_limit() -> usize {
    2
}

impl HttpConfig {
    /// 从环境变量加载
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::with_prefix("AXIOM"))
            .build()?
            .try_deserialize()
    }
    
    /// 从配置文件加载
    pub fn from_file(path: &str) -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::File::with_name(path))
            .build()?
            .try_deserialize()
    }
}

// 修改 build() 函数
pub fn build() -> Router {
    build_with_config(HttpConfig::default())
}

pub fn build_with_config(config: HttpConfig) -> Router {
    let mut router = Router::new();
    
    // 收集路由...
    
    router
        .layer(DefaultBodyLimit::max(config.body_limit_mb * 1024 * 1024))
        .layer(CorsLayer::new().allow_origins(/* config.cors_origins */))
}
```

**使用示例**:
```rust
// 使用环境变量
#[tokio::main]
async fn main() {
    // 从环境变量读取配置
    // AXIOM_HOST=127.0.0.1
    // AXIOM_PORT=8080
    // AXIOM_BODY_LIMIT_MB=10
    let config = HttpConfig::from_env().unwrap_or_default();
    
    let app = axiom::http::build_with_config(config);
    
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    
    println!("Server running on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}

// 使用配置文件
#[tokio::main]
async fn main() {
    // config/axiom.toml
    let config = HttpConfig::from_file("config/axiom").unwrap();
    let app = axiom::http::build_with_config(config);
    // ...
}
```

**配置文件示例**:
```toml
# config/axiom.toml
host = "0.0.0.0"
port = 3000
cors_origins = ["http://localhost:5173", "https://example.com"]
body_limit_mb = 5
```

**影响范围**:
- [ ] PRD.md 新增非功能需求
- [ ] TDD.md 第 8 节
- [ ] Task.md 新增 TASK-035

**新增任务**:
```markdown
#### TASK-035: 配置管理系统 🟡 P2 ⏳ 待开发
**描述**: 实现运行时配置管理

**子任务**:
- [ ] 定义配置结构
- [ ] 实现环境变量加载
- [ ] 实现配置文件加载
- [ ] 修改 build() 函数
- [ ] 编写配置文档

**预估工时**: 8 小时
```

**状态**: ⏳ 待修正

---

### E-012: 文档术语不一致 ⏳ 待修正

**受影响文档**: 所有文档  
**严重程度**: 🟡 中等  
**发现时间**: 2025-01-01

**问题描述**:
- 术语在不同文档间不一致
- 影响阅读体验

**术语统一表**:

| 概念 | ❌ 错误/不一致 | ✅ 统一使用 |
|------|---------------|------------|
| 使用者 | 用户、客户、开发者（混用） | **SDK 开发者** |
| 接口定义 | API、接口、函数（混用） | **接口** |
| 测试项 | 测试用例、测试点、测试场景 | **测试用例** |
| 验收项 | 验收场景、验收点 | **验收场景** |
| 工作量 | 工时、时间、工作量 | **预估工时** |
| 协议 | 协议、传输协议、服务协议 | **协议** |
| 特性 | Feature、特性、功能开关 | **Feature** (英文) |
| 框架 | 框架、库、组件 | **库组件** |

**修正示例**:
```markdown
# ❌ 之前
PRD: "作为用户..."
TDD: "客户端请求..."
UAT: "开发者验收..."

# ✅ 统一后
所有文档: "作为 SDK 开发者..."
```

**影响范围**:
- [ ] 所有文档全文

**状态**: ⏳ 待修正

---

## 🟢 轻微问题修正

### E-013: 文档格式不统一 ⏳ 待修正

**受影响文档**: 所有文档  
**严重程度**: 🟢 轻微  
**发现时间**: 2025-01-01

**问题描述**:
- 部分使用中文标点，部分使用英文标点
- 代码块语言标记不一致

**修正规范**:
```markdown
# ✅ 标点符号规范
- 中文内容使用中文标点：这是一个示例。
- 英文内容使用英文标点: This is an example.
- 代码、路径、命令使用英文标点: `path/to/file.rs`

# ✅ 代码块语言标记统一
\`\`\`rust  (不是 rs)
\`\`\`toml  (不是 config)
\`\`\`bash  (不是 sh 或 shell)
\`\`\`json  (不是 js)
```

**状态**: ⏳ 待修正

---

### E-014: 代码示例缺少注释 ⏳ 待修正

**修正示例**:
```rust
// ❌ 之前
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    Ok(User { id, name: "Alice".to_string() })
}

// ✅ 修正后
/// 获取用户信息
/// 
/// # 参数
/// - `id`: 用户 ID
/// 
/// # 返回
/// - `Ok(User)`: 成功返回用户信息
/// - `Err(ApiError)`: 用户不存在或其他错误
#[service_api(
    name = "get_user",        // 接口名称
    version = "v1",            // API 版本
    path = "/users/:id",       // HTTP 路径（:id 为路径参数）
    method = "GET",            // HTTP 方法
    tool_name = "get_user",    // MCP 工具名（MCP feature 启用时使用）
    description = "Get user by ID"  // 工具描述
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    // 这里是业务逻辑实现
    // 实际项目中应该从数据库查询
    Ok(User { 
        id, 
        name: "Alice".to_string() 
    })
}
```

**状态**: ⏳ 待修正

---

### E-015: README 内容过于简单 ⏳ 待修正

**修正方案**:
```markdown
# Axiom

[![Crates.io](https://img.shields.io/crates/v/axiom.svg)](https://crates.io/crates/axiom)
[![Documentation](https://docs.rs/axiom/badge.svg)](https://docs.rs/axiom)
[![License](https://img.shields.io/crates/l/axiom.svg)](LICENSE)

Multi-protocol SDK framework with unified macro configuration.

## Features

- ✨ **Unified Configuration**: Define APIs once, deploy to multiple protocols
- 🚀 **Zero-Cost Abstraction**: Unused protocols generate no code
- 🔧 **Compile-Time Safety**: Type-safe with full Rust type system support
- 📦 **Library Component**: Integrate into existing projects
- 🎯 **Protocol Support**: HTTP (Axum) and MCP out of the box

## Quick Start

### Installation

\`\`\`toml
[dependencies]
axiom = { version = "0.1", features = ["http"] }
tokio = { version = "1", features = ["full"] }
\`\`\`

### Example

\`\`\`rust
use axiom::prelude::*;

#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user",
    description = "Get user by ID"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    Ok(User { id, name: "Alice".to_string() })
}

#[tokio::main]
async fn main() {
    let app = axiom::http::build();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
\`\`\`

## Features

| Feature | Description |
|---------|-------------|
| `http` | HTTP server support via Axum |
| `mcp` | MCP server support |
| `streaming` | SSE streaming support |
| `timestamp` | Auto-add timestamp to responses |
| `logging` | Tracing integration |
| `full` | All features enabled |

## Documentation

- [API Documentation](https://docs.rs/axiom)
- [User Guide](docs/guide.md)
- [Examples](examples/)

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))
```

**状态**: ⏳ 待修正

---

### E-016: 缺少贡献指南 ⏳ 待修正

**新增 CONTRIBUTING.md**:
```markdown
# Contributing to Axiom

Thank you for your interest in contributing!

## Development Setup

\`\`\`bash
# Clone repository
git clone https://github.com/username/axiom.git
cd axiom

# Run tests
cargo test --all-features

# Run benchmarks
cargo bench --features http
\`\`\`

## Pull Request Process

1. Create a feature branch
2. Write tests for new features
3. Ensure all tests pass
4. Update documentation
5. Submit PR

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Follow Rust naming conventions

## Testing

- Unit tests: > 80% coverage
- Integration tests for new features
- Benchmark tests for performance changes
```

**状态**: ⏳ 待修正

---

## 修正优先级总览

| 优先级 | 问题数 | 预计工时 | 状态 |
|--------|--------|---------|------|
| 🔴 严重 | 8 | 60 小时 | ⏳ 待修正 |
| 🟡 中等 | 4 | 30 小时 | ⏳ 待修正 |
| 🟢 轻微 | 4 | 10 小时 | ⏳ 待修正 |
| **总计** | **16** | **100 小时** | ⏳ 待修正 |

---

## 修正执行计划

### Week 1: 严重问题修正 (40 小时)
- [ ] E-001: 更换 MCP 库
- [ ] E-002: 更新 Axum 版本
- [ ] E-003: 添加缺失依赖
- [ ] E-004: 实现输入验证
- [ ] E-008: 锁定依赖版本

### Week 2: 严重问题修正 + 中等问题 (30 小时)
- [ ] E-005: 统一 Feature 检查
- [ ] E-006: 完善流式错误处理
- [ ] E-007: 补充边界测试
- [ ] E-009: 调整任务优先级

### Week 3: 中等问题 + 轻微问题 (30 小时)
- [ ] E-010: 简化模块前缀
- [ ] E-011: 添加配置管理
- [ ] E-012: 统一文档术语
- [ ] E-013 ~ E-016: 文档优化

---

## 验收标准

### 严重问题验收
- [ ] 所有依赖版本明确
- [ ] 使用官方 MCP SDK
- [ ] Axum 0.8 API 正确使用
- [ ] 输入验证覆盖所有端点
- [ ] Feature 检查逻辑统一
- [ ] 流式响应有完整错误处理
- [ ] 边界测试覆盖率 > 90%

### 中等问题验收
- [ ] 配置可通过环境变量/文件加载
- [ ] 模块前缀实现简洁清晰
- [ ] 文档术语完全统一

### 轻微问题验收
- [ ] 文档格式统一
- [ ] 代码示例有注释
- [ ] README 内容完整
- [ ] 有贡献指南

---

## 修正状态跟踪

使用以下命令更新状态:
- `⏳ 待修正` → `🚧 修正中` → `✅ 已修正` → `✔️ 已验证`

**当前进度**: 0/16 (0%)
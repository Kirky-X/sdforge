# 用户验收文档 (UAT)

## Axiom - Multi-Protocol SDK Framework

**版本**: v1.2 (修复版)  
**日期**: 2025-01-01

---

## 1. 验收范围

本文档定义框架功能的用户验收标准，从**SDK 开发者**的角度验证系统可用性。

---

## 2. 验收场景

### 2.1 基础集成 - 仅 HTTP 服务 ⏳ 待验收

#### UAT-001: 快速集成 HTTP 服务

**用户角色**: SDK 开发者  
**业务目标**: 5 分钟内将现有函数暴露为 HTTP API

**验收步骤**:

1. [ ] 在现有项目中添加依赖:

```toml
[dependencies]
axiom = { version = "0.1", features = ["http"] }
tokio = { version = "1", features = ["full"] }
```

2. [ ] 为现有函数添加宏:

```rust
use axiom::prelude::*;

// 现有业务函数
async fn get_user_impl(id: u64) -> Result<User, ApiError> {
    // 业务逻辑
    Ok(User { id, name: "Alice".to_string() })
}

// 添加宏
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user",  // 虽然定义了，但不会生成 MCP 代码
    description = "Get user by ID"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    get_user_impl(id).await
}
```

3. [ ] 启动服务:

```rust
#[tokio::main]
async fn main() {
    let app = axiom::http::build();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
```

4. [ ] 测试接口:

```bash
curl http://localhost:3000/api/v1/users/123
# 期望输出: {"success":true,"data":{"id":123,"name":"Alice"}}
```

**验收标准**:

- [ ] 从添加依赖到运行 < 5 分钟
- [ ] 无需编写路由代码
- [ ] 响应格式符合规范
- [ ] 二进制文件不包含 MCP 相关代码

**状态**: ⏳ 待验收

---

### 2.2 基础集成 - 仅 MCP 服务 ⏳ 待验收

#### UAT-002: 快速创建 MCP 工具服务

**用户角色**: AI 应用开发者  
**业务目标**: 将现有函数暴露为 MCP 工具

**验收步骤**:

1. [ ] 添加依赖（仅 MCP）:

```toml
[dependencies]
axiom = { version = "0.1", features = ["mcp"] }
tokio = { version = "1", features = ["full"] }
```

2. [ ] 定义工具:

```rust
use axiom::prelude::*;

#[service_api(
    name = "search",
    version = "v1",        // HTTP 参数，但不会使用
    path = "/search",      // HTTP 参数，但不会使用
    method = "GET",        // HTTP 参数，但不会使用
    tool_name = "search_documentation",
    description = "Search through project documentation"
)]
async fn search_docs(
    query: String,
    max_results: Option<u32>,
) -> Result<Vec<Document>, ApiError> {
    // 实现搜索逻辑
    Ok(vec![
        Document { title: "Getting Started".to_string(), content: "...".to_string() },
        Document { title: "API Reference".to_string(), content: "...".to_string() },
    ])
}
```

3. [ ] 启动 MCP 服务:

```rust
#[tokio::main]
async fn main() {
    let server = axiom::mcp::build().await;
    println!("MCP server running");
    server.run().await.unwrap();
}
```

4. [ ] 在 Claude Desktop 配置:

```json
{
  "mcpServers": {
    "my-docs": {
      "command": "/path/to/my-mcp-server",
      "args": []
    }
  }
}
```

5. [ ] 在 Claude 中测试工具

**验收标准**:

- [ ] MCP 工具在 Claude 中可见
- [ ] 工具描述清晰准确
- [ ] 参数 Schema 自动生成正确
- [ ] 调用返回预期结果
- [ ] 二进制文件不包含 HTTP 相关代码

**状态**: ⏳ 待验收

---

### 2.3 双协议支持 ⏳ 待验收

#### UAT-003: 同一函数暴露为两种协议

**用户角色**: 平台开发者  
**业务目标**: 同一业务逻辑既可通过 HTTP 访问，也可作为 MCP 工具

**验收步骤**:

1. [ ] 添加两种 feature:

```toml
[dependencies]
axiom = { version = "0.1", features = ["http", "mcp"] }
```

2. [ ] 定义接口（统一配置）:

```rust
#[service_api(
    name = "analyze_code",
    version = "v1",
    path = "/analyze",
    method = "POST",
    tool_name = "analyze_code",
    description = "Analyze code quality"
)]
async fn analyze_code(
    code: String,
    language: String,
) -> Result<AnalysisReport, ApiError> {
    // 分析逻辑
    Ok(AnalysisReport {
        score: 85,
        issues: vec!["Line 10: unused variable".to_string()],
    })
}
```

3. [ ] 启动两个服务:

```rust
#[tokio::main]
async fn main() {
    // 启动 HTTP 服务
    tokio::spawn(async {
        let app = axiom::http::build();
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
    
    // 启动 MCP 服务
    let server = axiom::mcp::build().await;
    server.run().await.unwrap();
}
```

4. [ ] 测试 HTTP 接口:

```bash
curl -X POST http://localhost:3000/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{"code":"fn main() {}","language":"rust"}'
```

5. [ ] 测试 MCP 工具（在 Claude 中调用）

**验收标准**:

- [ ] HTTP 和 MCP 都能正常工作
- [ ] 两种协议返回结果一致
- [ ] 无需重复实现业务逻辑
- [ ] 统一的配置管理

**状态**: ⏳ 待验收

---

### 2.4 模块化组织 ⏳ 待验收

#### UAT-004: 使用模块前缀组织 API

**用户角色**: API 架构师  
**业务目标**: 将 API 按业务模块组织，使用不同的 URL 前缀

**验收步骤**:

1. [ ] 定义模块化结构:

```rust
// 认证模块
#[service_module(prefix = "/auth")]
mod auth {
    use axiom::prelude::*;
    
    #[service_api(
        name = "login",
        version = "v1",
        path = "/login",
        method = "POST",
        tool_name = "user_login",
        description = "User login"
    )]
    async fn login(
        username: String,
        password: String,
    ) -> Result<Token, ApiError> {
        // 登录逻辑
        Ok(Token { token: "jwt-token".to_string() })
    }
    
    #[service_api(
        name = "logout",
        version = "v1",
        path = "/logout",
        method = "POST",
        tool_name = "user_logout"
    )]
    async fn logout(token: String) -> Result<(), ApiError> {
        // 登出逻辑
        Ok(())
    }
}

// 用户模块
#[service_module(prefix = "/users")]
mod users {
    use axiom::prelude::*;
    
    #[service_api(
        name = "get_profile",
        version = "v1",
        path = "/profile",
        method = "GET",
        tool_name = "get_user_profile"
    )]
    async fn get_profile(user_id: u64) -> Result<Profile, ApiError> {
        // 获取个人资料
        Ok(Profile { name: "Alice".to_string() })
    }
}
```

2. [ ] 验证路径组合:

```bash
# 认证模块
curl -X POST http://localhost:3000/auth/api/v1/login
curl -X POST http://localhost:3000/auth/api/v1/logout

# 用户模块
curl http://localhost:3000/users/api/v1/profile?user_id=123
```

**验收标准**:

- [ ] 模块前缀正确应用
- [ ] URL 结构清晰易懂
- [ ] 不同模块的接口隔离
- [ ] 自动生成的路径符合预期

**状态**: ⏳ 待验收

---

### 2.5 Feature 控制功能 ⏳ 待验收

#### UAT-005: Timestamp 特性

**用户角色**: API 开发者  
**业务目标**: 为所有响应自动添加时间戳，便于调试

**验收步骤**:

1. [ ] 启用 timestamp feature:

```toml
[dependencies]
axiom = { version = "0.1", features = ["http", "timestamp"] }
```

2. [ ] 定义接口（无需额外代码）:

```rust
#[service_api(
    name = "get_data",
    version = "v1",
    path = "/data",
    method = "GET",
    tool_name = "get_data"
)]
async fn get_data() -> Result<Data, ApiError> {
    Ok(Data { value: 42 })
}
```

3. [ ] 验证响应包含时间戳:

```bash
curl http://localhost:3000/api/v1/data
# 输出: {"success":true,"data":{"value":42},"timestamp":1704067200}
```

4. [ ] 禁用 feature 并重新编译:

```toml
[dependencies]
axiom = { version = "0.1", features = ["http"] }  # 移除 timestamp
```

5. [ ] 验证响应不包含时间戳:

```bash
curl http://localhost:3000/api/v1/data
# 输出: {"success":true,"data":{"value":42}}
```

**验收标准**:

- [ ] 启用 feature 后自动添加 timestamp 字段
- [ ] 禁用 feature 后完全不包含该字段
- [ ] 无运行时性能损失
- [ ] 无需修改业务代码

**状态**: ⏳ 待验收

---

#### UAT-006: Logging 特性

**用户角色**: 运维开发者  
**业务目标**: 自动记录所有请求日志

**验收步骤**:

1. [ ] 启用 logging feature:

```toml
[dependencies]
axiom = { version = "0.1", features = ["http", "logging"] }
```

2. [ ] 配置日志级别:

```bash
RUST_LOG=info cargo run
```

3. [ ] 发送请求并观察日志:

```bash
curl http://localhost:3000/api/v1/users/123
```

4. [ ] 验证日志输出:

```
INFO request{method=GET uri=/api/v1/users/123}: started
INFO request{method=GET uri=/api/v1/users/123}: completed in 5ms status=200
```

**验收标准**:

- [ ] 自动记录所有请求
- [ ] 日志包含关键信息（method, uri, duration, status）
- [ ] 日志格式结构化
- [ ] 支持不同日志级别（debug/info/warn/error）

**状态**: ⏳ 待验收

---

### 2.6 复杂数据类型处理 ⏳ 待验收

#### UAT-007: 嵌套结构序列化

**用户角色**: API 开发者  
**业务目标**: 处理复杂的嵌套数据结构

**验收步骤**:

1. [ ] 定义复杂数据结构:

```rust
#[derive(Debug, Serialize, Deserialize)]
struct CreateOrderRequest {
    customer: Customer,
    items: Vec<OrderItem>,
    shipping_address: Address,
    metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Customer {
    id: u64,
    name: String,
    email: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OrderItem {
    product_id: u64,
    quantity: u32,
    price: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Address {
    street: String,
    city: String,
    country: String,
    postal_code: String,
}
```

2. [ ] 定义 API:

```rust
#[service_api(
    name = "create_order",
    version = "v1",
    path = "/orders",
    method = "POST",
    tool_name = "create_order",
    description = "Create a new order"
)]
async fn create_order(
    request: CreateOrderRequest,
) -> Result<Order, ApiError> {
    // 处理订单
    Ok(Order { id: 12345, status: "pending".to_string() })
}
```

3. [ ] 发送复杂 JSON:

```bash
curl -X POST http://localhost:3000/api/v1/orders \
  -H "Content-Type: application/json" \
  -d '{
    "customer": {
      "id": 1,
      "name": "Alice",
      "email": "alice@example.com"
    },
    "items": [
      {"product_id": 101, "quantity": 2, "price": 29.99},
      {"product_id": 102, "quantity": 1, "price": 49.99}
    ],
    "shipping_address": {
      "street": "123 Main St",
      "city": "San Francisco",
      "country": "US",
      "postal_code": "94102"
    },
    "metadata": {
      "source": "web",
      "campaign": "summer_sale"
    }
  }'
```

**验收标准**:

- [ ] 嵌套结构正确反序列化
- [ ] 所有字段类型正确
- [ ] 无需手动转换代码
- [ ] 验证错误清晰（如缺少字段）

**状态**: ⏳ 待验收

---

### 2.7 流式响应 ⏳ 待验收

#### UAT-008: 实时日志流

**用户角色**: 运维开发者  
**业务目标**: 实现实时日志推送

**验收步骤**:

1. [ ] 启用 streaming feature:

```toml
[dependencies]
axiom = { version = "0.1", features = ["http", "streaming"] }
```

2. [ ] 定义流式 API:

```rust
use tokio_stream::{Stream, StreamExt};

#[service_api(
    name = "stream_logs",
    version = "v1",
    path = "/logs",
    method = "GET",
    stream = true
)]
async fn stream_logs(
    service: String,
) -> Result<impl Stream<Item = LogEntry>, ApiError> {
    Ok(tokio_stream::iter(vec![
        LogEntry { timestamp: 1704067200, level: "INFO", message: "Service started".to_string() },
        LogEntry { timestamp: 1704067201, level: "DEBUG", message: "Processing request".to_string() },
        LogEntry { timestamp: 1704067202, level: "INFO", message: "Request completed".to_string() },
    ]))
}
```

3. [ ] 使用 EventSource 客户端:

```javascript
const eventSource = new EventSource('http://localhost:3000/api/v1/logs?service=web');
eventSource.onmessage = (event) => {
  const log = JSON.parse(event.data);
  console.log(`[${log.level}] ${log.message}`);
};
```

4. [ ] 或使用 curl:

```bash
curl -N http://localhost:3000/api/v1/logs?service=web
# 输出 SSE 流:
# data: {"timestamp":1704067200,"level":"INFO","message":"Service started"}
# data: {"timestamp":1704067201,"level":"DEBUG","message":"Processing request"}
# data: {"timestamp":1704067202,"level":"INFO","message":"Request completed"}
```

**验收标准**:

- [ ] 持续接收数据流
- [ ] 每条数据格式正确
- [ ] 连接断开后自动重连
- [ ] 支持长时间连接

**状态**: ⏳ 待验收

---

### 2.8 错误处理 ⏳ 待验收

#### UAT-009: 统一错误响应

**用户角色**: 前端开发者  
**业务目标**: 统一的错误处理便于客户端处理

**验收步骤**:

1. [ ] 定义带错误的接口:

```rust
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    if id == 0 {
        return Err(ApiError::InvalidInput {
            message: "User ID must be greater than 0".to_string(),
            field: Some("id".to_string()),
        });
    }
    
    if id == 999 {
        return Err(ApiError::NotFound {
            resource: "user".to_string(),
        });
    }
    
    Ok(User { id, name: "Alice".to_string() })
}
```

2. [ ] 测试不同错误:

```bash
# 400 错误
curl http://localhost:3000/api/v1/users/0
# 输出: {"success":false,"error":{"code":"INVALID_INPUT","message":"User ID must be greater than 0","details":{"field":"id"},"http_status":400}}

# 404 错误
curl http://localhost:3000/api/v1/users/999
# 输出: {"success":false,"error":{"code":"NOT_FOUND","message":"Resource not found: user","details":null,"http_status":404}}
```

**验收标准**:

- [ ] 错误格式统一
- [ ] HTTP 状态码正确
- [ ] 错误码清晰易懂
- [ ] 包含详细信息便于调试

**状态**: ⏳ 待验收

---

### 2.9 版本管理 ⏳ 待验收

#### UAT-010: API 多版本共存

**用户角色**: API 维护者  
**业务目标**: 平滑升级 API，旧版本继续可用

**验收步骤**:

1. [ ] 实现 v1 和 v2:

```rust
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user_v1"
)]
async fn get_user_v1(id: u64) -> Result<UserV1, ApiError> {
    Ok(UserV1 {
        id,
        name: "Alice".to_string(),
    })
}

#[service_api(
    name = "get_user",
    version = "v2",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user_v2"
)]
async fn get_user_v2(id: u64) -> Result<UserV2, ApiError> {
    Ok(UserV2 {
        id,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),  // v2 新增字段
        created_at: 1704067200,
    })
}
```

2. [ ] 测试两个版本:

```bash
# v1
curl http://localhost:3000/api/v1/users/123
# 输出: {"success":true,"data":{"id":123,"name":"Alice"}}

# v2
curl http://localhost:3000/api/v2/users/123
# 输出: {"success":true,"data":{"id":123,"name":"Alice","email":"alice@example.com","created_at":1704067200}}
```

**验收标准**:

- [ ] 两个版本同时可用
- [ ] 路径自动隔离
- [ ] 返回数据结构不同
- [ ] 旧版本不受新版本影响

**状态**: ⏳ 待验收

---

### 2.10 性能验收 ⏳ 待验收

#### UAT-011: 3000 QPS 目标

**用户角色**: 技术负责人  
**业务目标**: 验证性能指标

**验收步骤**:

1. [ ] 部署 100 个接口的服务
2. [ ] 使用 wrk 压测:

```bash
wrk -t 8 -c 100 -d 60s http://localhost:3000/api/v1/health
```

3. [ ] 记录结果:

```
Requests/sec: 6543.21
Latency:
  50%: 15.23ms
  90%: 45.67ms
  99%: 87.34ms
```

**验收标准**:

- [ ] QPS > 3000
- [ ] P50 延迟 < 30ms
- [ ] P99 延迟 < 150ms
- [ ] 60秒内无错误

**状态**: ⏳ 待验收

---

## 3. 开发体验验收

### 3.1 编译错误友好性 ⏳ 待验收

#### UAT-012: 友好的错误提示

**验收步骤**:

1. [ ] 故意写错误代码:

```rust
#[service_api(
    name = "test",
    version = "v1",
    method = "INVALID_METHOD"  // 错误的方法
)]
async fn test() {}
```

2. [ ] 验证错误信息:

```
error: Invalid HTTP method: INVALID_METHOD
  --> src/main.rs:5:13
   |
5  |     method = "INVALID_METHOD"
   |              ^^^^^^^^^^^^^^^^
   |
   = note: Allowed values: GET, POST, PUT, DELETE, PATCH
```

**验收标准**:

- [ ] 错误信息清晰
- [ ] 指向正确位置
- [ ] 提供修复建议
- [ ] 不显示内部宏代码

**状态**: ⏳ 待验收

---

#### UAT-013: Feature 依赖错误提示

**验收步骤**:

1. [ ] 尝试使用未启用的 feature:

```toml
[dependencies]
axiom = { version = "0.1" }  # 未启用任何 feature
```

```rust
#[service_api(
    name = "test",
    version = "v1",
    path = "/test",
    method = "GET"
)]
async fn test() {}
```

2. [ ] 验证编译错误:

```
error: At least one protocol feature (http or mcp) must be enabled
  --> src/main.rs:1:1
   |
   = help: Add `features = ["http"]` or `features = ["mcp"]` to your Cargo.toml
```

**验收标准**:

- [ ] 清楚说明缺少的 feature
- [ ] 提供修复方法

**状态**: ⏳ 待验收

---

### 3.2 文档完整性 ⏳ 待验收

#### UAT-014: 文档可用性

**验收清单**:

- [ ] README 包含快速开始（< 5 分钟可运行）
- [ ] 所有宏参数有文档说明
- [ ] 示例代码可直接运行
- [ ] Feature 说明清晰
- [ ] 常见问题 FAQ
- [ ] API 文档完整（rustdoc）

**状态**: ⏳ 待验收

---

## 4. 集成验收

### 4.1 真实项目集成 ⏳ 待验收

#### UAT-015: 集成到现有项目

**用户角色**: 项目负责人  
**业务目标**: 在现有项目中集成 Axiom

**验收步骤**:

1. [ ] 选择一个现有 Rust 项目
2. [ ] 添加 Axiom 依赖
3. [ ] 为 3-5 个函数添加宏
4. [ ] 编译并运行
5. [ ] 验证功能正常

**验收标准**:

- [ ] 集成过程顺利
- [ ] 不影响现有代码
- [ ] 编译时间增加 < 20%
- [ ] 二进制体积增加合理

**状态**: ⏳ 待验收

---

## 5. 验收通过标准

### 5.1 功能完整性

- [ ] 所有 UAT 场景通过（15 个）
- [ ] 无阻塞性缺陷
- [ ] 性能指标达标

### 5.2 开发体验

- [ ] 错误提示友好
- [ ] 文档完整清晰
- [ ] 示例可运行

### 5.3 质量指标

- [ ] 测试覆盖率 > 80%
- [ ] 所有 feature 组合编译通过
- [ ] 代码审查通过

---

## 6. 验收签署

| 角色       | 姓名 | 签署日期 | 状态     |
| ---------- | ---- | -------- | -------- |
| 产品负责人 |      |          | ⏳ 待验收 |
| 技术负责人 |      |          | ⏳ 待验收 |
| QA 负责人  |      |          | ⏳ 待验收 |
| 用户代表   |      |          | ⏳ 待验收 |
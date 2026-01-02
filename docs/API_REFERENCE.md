<div align="center">

# 📘 API 参考文档

### Axiom 多协议 SDK 框架完整 API 文档

[🏠 首页](../README.md) • [📖 用户指南](USER_GUIDE.md) • [🏗️ 架构](ARCHITECTURE.md)

---

</div>

## 📋 目录

- [概述](#概述)
- [核心 API](#核心-api)
  - [初始化](#初始化)
  - [配置](#配置)
  - [服务构建](#服务构建)
  - [错误处理](#错误处理)
- [宏 API](#宏-api)
- [协议支持](#协议支持)
- [类型定义](#类型定义)
- [示例](#示例)

---

## 概述

<div align="center">

### 🎯 API 设计原则

</div>

<table>
<tr>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/easy.png" width="64"><br>
<b>简单</b><br>
直观易用
</td>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/security-checked.png" width="64"><br>
<b>安全</b><br>
类型安全，默认安全
</td>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/module.png" width="64"><br>
<b>可组合</b><br>
轻松构建复杂工作流
</td>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/documentation.png" width="64"><br>
<b>文档完善</b><br>
全面文档支持
</td>
</tr>
</table>

---

## 核心 API

### 初始化

<div align="center">

#### 🚀 快速开始

</div>

---

#### `axiom::http::build()`

构建 HTTP 服务。

<table>
<tr>
<td width="30%"><b>签名</b></td>
<td width="70%">

```rust
pub fn build() -> Result<Router, Infallible>
```

</td>
</tr>
<tr>
<td><b>描述</b></td>
<td>构建 HTTP 路由器，自动注册所有使用 `#[service_api]` 宏定义的端点。</td>
</tr>
<tr>
<td><b>返回</b></td>
<td><code>Result&lt;Router, Infallible&gt;</code> - 始终返回 Ok</td>
</tr>
</table>

**示例：**

```rust
use axiom::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = axiom::http::build()?;
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("🚀 Server running on http://localhost:3000");
    
    axum::serve(listener, app).await?;
    Ok(())
}
```

---

#### `axiom::mcp::build()`

构建 MCP 服务。

<table>
<tr>
<td width="30%"><b>签名</b></td>
<td width="70%">

```rust
pub async fn build() -> Result<Server, Box<dyn std::error::Error>>
```

</td>
</tr>
<tr>
<td><b>描述</b></td>
<td>构建 MCP 服务器，自动注册所有使用 `#[service_api]` 宏定义的工具。</td>
</tr>
<tr>
<td><b>返回</b></td>
<td><code>Result&lt;Server, Box&lt;dyn Error&gt;&gt;</code></td>
</tr>
</table>

**示例：**

```rust
use axiom::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = axiom::mcp::build().await?;
    
    // MCP 服务通常通过 stdio 运行
    server.run().await?;
    Ok(())
}
```

---

### 配置

<div align="center">

#### ⚙️ 配置选项

</div>

---

#### Feature 配置

<table>
<tr>
<th>Feature</th>
<th>说明</th>
<th>默认值</th>
</tr>
<tr>
<td><code>http</code></td>
<td>启用 HTTP 服务器支持</td>
<td>可选</td>
</tr>
<tr>
<td><code>mcp</code></td>
<td>启用 MCP 协议支持</td>
<td>可选</td>
</tr>
<tr>
<td><code>streaming</code></td>
<td>启用 SSE 流式响应</td>
<td>禁用</td>
</tr>
<tr>
<td><code>timestamp</code></td>
<td>启用响应时间戳</td>
<td>禁用</td>
</tr>
<tr>
<td><code>logging</code></td>
<td>启用请求日志</td>
<td>禁用</td>
</tr>
<tr>
<td><code>security</code></td>
<td>启用安全认证模块</td>
<td>禁用</td>
</tr>
<tr>
<td><code>cache</code></td>
<td>启用响应缓存</td>
<td>禁用</td>
</tr>
</table>

---

### 服务构建

<div align="center">

#### 🏗️ 构建服务

</div>

---

#### `#[service_api]` 宏

定义 API 端点的核心宏。

<table>
<tr>
<td width="30%"><b>属性参数</b></td>
<td width="70%">

```rust
#[service_api(
    name = "api_name",
    version = "v1",
    path = "/path/:param",
    method = "GET",
    tool_name = "tool_name",
    description = "API 描述"
)]
```

</td>
</tr>
</table>

**必需参数：**
- `name` - API 名称（唯一标识符）
- `version` - API 版本（如 "v1"）

**HTTP 参数（启用 http feature）：**
- `path` - 路由路径
- `method` - HTTP 方法（GET, POST, PUT, DELETE）

**MCP 参数（启用 mcp feature）：**
- `tool_name` - MCP 工具名称
- `description` - 工具描述

**示例：**

```rust
use axiom::prelude::*;

#[derive(serde::Deserialize)]
struct UserId {
    id: u64,
}

#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user",
    description = "根据 ID 获取用户信息"
)]
async fn get_user(id: UserId) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id.id,
        "name": format!("User {}", id.id)
    }))
}
```

---

#### `#[service_module]` 宏

为相关 API 设置路径前缀。

<table>
<tr>
<td width="30%"><b>属性参数</b></td>
<td width="70%">

```rust
#[service_module(prefix = "/api_prefix")]
```

</td>
</tr>
</table>

**示例：**

```rust
use axiom::prelude::*;

#[service_module(prefix = "/auth")]
mod auth_api {
    use axiom::prelude::*;
    
    #[service_api(
        name = "login",
        version = "v1",
        path = "/login",
        method = "POST",
        tool_name = "login"
    )]
    async fn login(req: LoginRequest) -> Result<Token, ApiError> {
        // 实现登录逻辑
    }
}
```

---

### 错误处理

<div align="center">

#### 🚨 错误类型

</div>

#### `ApiError` 枚举

```rust
use axiom::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {resource}")]
    NotFound { resource: String },
    
    #[error("Invalid input: {message}")]
    InvalidInput { 
        message: String,
        field: Option<String>,
        value: Option<serde_json::Value>,
    },
    
    #[error("Unauthorized")]
    Unauthorized,
    
    #[error("Internal error: {message}")]
    Internal { message: String },
    
    #[error("Rate limited")]
    RateLimited,
}
```

**常用错误变体：**

| 变体 | HTTP 状态码 | 使用场景 |
|------|-------------|----------|
| `ApiError::NotFound` | 404 | 资源不存在 |
| `ApiError::InvalidInput` | 400 | 输入验证失败 |
| `ApiError::Unauthorized` | 401 | 未认证 |
| `ApiError::Internal` | 500 | 服务器内部错误 |
| `ApiError::RateLimited` | 429 | 请求频率超限 |

**错误处理示例：**

```rust
use axiom::prelude::*;

#[service_api(
    name = "create_user",
    version = "v1",
    path = "/users",
    method = "POST",
    tool_name = "create_user"
)]
async fn create_user(req: CreateUserRequest) -> Result<User, ApiError> {
    // 验证输入
    if req.username.is_empty() {
        return Err(ApiError::InvalidInput {
            message: "用户名不能为空".to_string(),
            field: Some("username".to_string()),
            value: None,
        });
    }
    
    // 业务逻辑
    let user = save_user(req).await?;
    Ok(user)
}
```

---

## 宏 API

<div align="center">

#### 🔧 宏参考

</div>

### `#[service_api]`

```rust
#[service_api(
    name = str,           // 必需：API 名称
    version = str,        // 必需：版本号
    path = str,           // HTTP：路由路径
    method = str,         // HTTP：HTTP 方法
    tool_name = str,      // MCP：工具名称
    description = str,    // MCP：描述
)]
```

### `#[service_module]`

```rust
#[service_module(prefix = str)]  // 必需：路径前缀
```

---

## 协议支持

### HTTP 协议

<div align="center">

#### 🌐 HTTP 端点格式

</div>

**路径规则：**
- 基础路径：`/api/{version}{path}`
- 带模块：`{module_prefix}/api/{version}{path}`

**示例：**

| 定义 | HTTP 端点 |
|------|----------|
| `#[service_api(path = "/hello")]` | GET /api/v1/hello |
| `#[service_module(prefix = "/auth")]` + path = "/login" | POST /auth/api/v1/login |

### MCP 协议

<div align="center">

#### 🤖 MCP 工具注册

</div>

**自动注册：**
- 函数名 → 工具名称
- 参数类型 → 工具输入模式
- 返回类型 → 工具输出模式

---

## 类型定义

### 核心类型

<table>
<tr>
<td width="50%">

**ServiceResponse**
```rust
pub struct ServiceResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: Option<i64>,
}
```

</td>
<td width="50%">

**ApiMetadata**
```rust
pub struct ApiMetadata {
    pub name: &'static str,
    pub version: &'static str,
    pub path: &'static str,
    pub method: HttpMethod,
}
```

</td>
</tr>
</table>

---

## 示例

<div align="center">

### 💡 常用示例

</div>

### 示例 1: 基础 HTTP API

```rust
use axiom::prelude::*;

#[service_api(
    name = "hello",
    version = "v1",
    path = "/hello",
    method = "GET",
    tool_name = "hello"
)]
async fn hello() -> Result<String, ApiError> {
    Ok("Hello, Axiom!".to_string())
}
```

### 示例 2: 带参数的 API

```rust
use axiom::prelude::*;

#[derive(serde::Deserialize)]
struct UserRequest {
    id: u64,
}

#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user"
)]
async fn get_user(req: UserRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": req.id,
        "name": format!("User {}", req.id)
    }))
}
```

### 示例 3: POST 请求

```rust
use axiom::prelude::*;

#[derive(serde::Deserialize)]
struct CreatePostRequest {
    title: String,
    content: String,
}

#[service_api(
    name = "create_post",
    version = "v1",
    path = "/posts",
    method = "POST",
    tool_name = "create_post"
)]
async fn create_post(req: CreatePostRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "title": req.title,
        "content": req.content
    }))
}
```

---

<div align="center">

**[📖 用户指南](USER_GUIDE.md)** • **[🏗️ 架构](ARCHITECTURE.md)** • **[🏠 首页](../README.md)**

由 Axiom 团队用 ❤️ 制作

[⬆ 返回顶部](#-api-参考文档)

</div>

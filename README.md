<div align="center" id="readme">

<img src="docs/asset/sdforge.png" alt="SDForge Logo" width="200" height="200">

[![Crates.io](https://img.shields.io/crates/v/sdforge)](https://crates.io/crates/sdforge) [![Documentation](https://img.shields.io/docsrs/sdforge)](https://docs.rs/sdforge) [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) [![Build Status](https://img.shields.io/github/actions/workflow/status/Kirky-X/sdforge/ci.yml?branch=main)](https://github.com/Kirky-X/sdforge/actions) [![Rust Version](https://img.shields.io/badge/rust-2021%2B-orange.svg)](https://www.rust-lang.org)

**SDForge** 是一个基于 Rust 的声明式 SDK 框架，利用过程宏从统一的函数注解自动生成多协议服务接口（HTTP + MCP）。其核心创新在于通过 Cargo features 进行编译时协议选择——未使用的协议将产生零编译代码。

</div>


## 📋 目录

<style>
.back-to-top {
  position: fixed;
  bottom: 20px;
  right: 20px;
  padding: 10px 20px;
  background-color: #007bff;
  color: white;
  text-decoration: none;
  border-radius: 5px;
  font-size: 14px;
  box-shadow: 0 2px 5px rgba(0,0,0,0.2);
  transition: background-color 0.3s;
  z-index: 1000;
}

.back-to-top:hover {
  background-color: #0056b3;
}

.doc-nav {
  padding: 10px 0;
  margin-top: 20px;
  border-top: 1px solid #e1e4e8;
}
</style>

- [✨ 特性](#features)
- [📦 安装](#installation)
- [🚀 快速开始](#quick-start)
- [⚙️ 特性系统](#feature-system)
- [💡 使用示例](#usage-examples)
- [📁 模块前缀](#module-prefixes)
- [🔢 版本管理](#version-management)
- [⚠️ 错误处理](#error-handling)
- [🛤️ 路径参数](#path-parameters)
- [🔨 构建与测试](#building-and-testing)
- [📚 文档](#documentation)
- [🔒 安全配置](#security-configuration)
- [⚡ 性能优化](#performance-optimization)
- [📜 OpenAPI 自动生成](#openapi-generation)
- [🔄 MCP 2026-07-28 迁移指南](#mcp-migration)
- [🚀 生产部署](#deployment-guide)
- [🐛 故障排除](#troubleshooting)
- [🤝 贡献](#contributing)
- [📜 许可证](#license)
- [📂 项目结构](#project-structure)
- [🔗 链接](#links)

---

<details open style="border-radius:8px; padding:16px; border:1px solid #E2E8F0;">
<summary><h2><span id="features">✨ 特性</span></h2></summary>

- **🎯 统一接口定义** - 针对 HTTP 和 MCP 协议的单一宏配置
- **⚡ 编译时协议选择** - 通过 Feature 控制代码生成，未使用的协议零运行时开销
- **🔒 类型安全** - 接口定义的编译时验证
- **🌐 多协议支持** - HTTP (Axum), MCP, gRPC, WebSocket, SSE 流式传输
- **🧩 模块化设计** - 基于 Feature 的架构，允许仅选择所需功能
- **🛡️ 安全特性** - 内置认证、限流和请求验证
- **💾 缓存** - 基于内存缓存（oxcache 0.3.2），无需外部数据库
- **🔧 配置管理** - 自包含的 TOML 配置（无需外部配置中心）
- **📊 版本控制** - 内置 API 版本管理
</details>

---

## <span id="installation">📦 安装</span>

将 SDForge 添加到你的 `Cargo.toml`:

```toml
[dependencies]
sdforge = { version = "0.3", features = ["http"] }
```

---

<details open style="border-radius:8px; padding:16px; border:1px solid #E2E8F0;">
<summary><h2><span id="quick-start">🚀 快速开始</span></h2></summary>

使用单个宏定义你的 API：

```rust
use sdforge::prelude::*;

#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user",
    description = "Get a user by ID"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    Ok(User { id, name: "Test".into() })
}

#[tokio::main]
async fn main() {
    let app = sdforge::http::build();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```
</details>

---

## <span id="feature-system">⚙️ 特性系统</span>

SDForge 使用 Cargo features 进行编译时协议选择和特性组合。

### 🔧 核心特性

| 特性         | 描述                                     | 依赖                                          |
|--------------|------------------------------------------|-----------------------------------------------|
| `http`       | HTTP 服务器 (Axum 0.8.8)                 | axum, tower, tower-http                        |
| `mcp`        | MCP 协议 (rmcp 0.16, 2026-07-28 规范)    | rmcp |
| `streaming`  | SSE 流式传输支持                         | tokio-stream, futures-util                    |
| `timestamp`  | 自动向响应添加时间戳                     | chrono                                        |
| `logging`    | 结构化请求日志                           | chrono, tokio                                 |
| `security`   | 安全特性 (认证, 限流)                    | http, cache, uuid, hmac, sha2, chrono, tokio, secrets, zeroize, subtle, once_cell, argon2, password-hash, rand, regex, oxcache/memory, bincode, hex, base64 |
| `websocket`  | WebSocket 支持                           | tokio-tungstenite, axum-extra                |
| `grpc`       | gRPC 支持                                | tonic, prost                                 |
| `cache`      | 缓存支持                                 | dep:http, oxcache/memory, async-trait         |
| `openapi`    | 自动 OpenAPI 3.1 规范生成                | utoipa, http                                  |
| `simd-json`  | SIMD 加速 JSON 序列化                   | simd-json                                     |
| `hex`        | 十六进制编码工具                        | hex                                           |
| `full`       | 启用所有运行时特性（不含 simd-json/hex 等工具特性） | -                                |

### 🔗 特性依赖

- `default`: [`http`]
- `mcp`: 独立协议（依赖外部 http crate 用于 stateless HTTP 头解析，不依赖 sdforge http feature）
- `streaming`: 需要 `http`
- `timestamp`: 无依赖
- `logging`: 无依赖
- `security`: 需要 `http`、`cache`
- `websocket`: 需要 `http`, `streaming`
- `grpc`: 需要 `http`
- `cache`: 独立（使用 http crate 类型，不依赖 sdforge http 特性）
- `openapi`: 需要 `http`

---

## <span id="usage-examples">💡 使用示例</span>

### 🌐 仅 HTTP

用于传统 REST API：

```toml
[dependencies]
sdforge = { version = "0.3", features = ["http"] }
```

### 🤖 仅 MCP

用于 AI 工具集成：

```toml
[dependencies]
sdforge = { version = "0.3", features = ["mcp"] }
```

### 🔄 双协议

从同一代码库同时暴露 HTTP 和 MCP：

```toml
[dependencies]
sdforge = { version = "0.3", features = ["http", "mcp"] }
```

### 🎯 全功能

启用所有功能：

```toml
[dependencies]
sdforge = { version = "0.3", features = ["full"] }
```

---

## <span id="module-prefixes">📁 模块前缀</span>

使用模块前缀对相关 API 进行分组，以便更好地组织：

```rust
#[service_module(prefix = "/auth")]
mod auth_api {
    use super::*;

    #[service_api(
        name = "login",
        version = "v1",
        path = "/login",
        method = "POST"
    )]
    async fn login(credentials: Credentials) -> Result<Token, ApiError> {
        // 实现
        Ok(Token::new())
    }

    #[service_api(
        name = "logout",
        version = "v1",
        path = "/logout",
        method = "POST"
    )]
    async fn logout() -> Result<(), ApiError> {
        // 实现
        Ok(())
    }
}
```

这将创建以下端点：
- `/auth/api/v1/login`
- `/auth/api/v1/logout`

---

## <span id="version-management">🔢 版本管理</span>

同时支持多个 API 版本：

```rust
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user_v1"
)]
async fn get_user_v1(id: u64) -> Result<UserV1, ApiError> {
    Ok(UserV1 { id, name: "John Doe".into() })
}

#[service_api(
    name = "get_user",
    version = "v2",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user_v2"
)]
async fn get_user_v2(id: u64) -> Result<UserV2, ApiError> {
    Ok(UserV2 { id, first_name: "John".into(), last_name: "Doe".into() })
}
```

这将创建版本化端点：
- `/api/v1/users/:id` → `get_user_v1`
- `/api/v2/users/:id` → `get_user_v2`

---

## <span id="error-handling">⚠️ 错误处理</span>

定义自定义错误类型并将其转换为 `ServiceError`：

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Validation failed: {field}")]
    ValidationError { field: String },

    #[error("Unauthorized access")]
    Unauthorized,
}

impl From<MyError> for ServiceError {
    fn from(err: MyError) -> Self {
        match err {
            MyError::NotFound { resource } => ServiceError::with_details(
                "NOT_FOUND",
                format!("Resource not found: {}", resource),
                serde_json::json!({ "resource": resource }),
                404,
            ),
            MyError::ValidationError { field } => ServiceError::with_details(
                "VALIDATION_ERROR",
                format!("Validation failed for field: {}", field),
                serde_json::json!({ "field": field }),
                400,
            ),
            MyError::Unauthorized => ServiceError::new(
                "UNAUTHORIZED",
                "Authentication required",
                401,
            ),
        }
    }
}
```

---

## <span id="path-parameters">🛤️ 路径参数</span>

使用 Rust 命名约定提取路径参数。宏会自动将路径段映射到函数参数：

```rust
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    // `id` 自动从 `/users/:id` 中提取
    Ok(User { id, name: "John".into() })
}

#[service_api(
    name = "get_comment",
    version = "v1",
    path = "/posts/:post_id/comments/:comment_id",
    method = "GET"
)]
async fn get_comment(
    post_id: u64,
    comment_id: u64
) -> Result<Comment, ApiError> {
    // 两个参数都从路径中提取
    Ok(Comment { post_id, comment_id, text: "Test".into() })
}
```

### 🔹 多个路径参数

用于嵌套资源：

```rust
#[service_api(
    name = "get_nested_resource",
    version = "v1",
    path = "/orgs/:org_id/projects/:project_id/tasks/:task_id",
    method = "GET"
)]
async fn get_task(
    org_id: u64,
    project_id: u64,
    task_id: u64
) -> Result<Task, ApiError> {
    Ok(Task { org_id, project_id, task_id, title: "Task".into() })
}
```

---

## <span id="building-and-testing">🔨 构建与测试</span>

### 🔧 构建命令

```bash
# 仅使用 HTTP 构建
cargo build --features http

# 仅使用 MCP 构建
cargo build --features mcp

# 使用所有特性构建
cargo build --features full

# 使用自定义特性集构建
cargo build --features "http,cache,security"

# 发布构建
cargo build --release --features http
```

### 🧪 测试命令

```bash
# 使用 HTTP 运行测试
cargo test --features http

# 使用 MCP 运行测试
cargo test --features mcp

# 使用双协议运行测试
cargo test --features "http,mcp"

# 使用所有特性运行测试
cargo test --features full

# 运行特定测试
cargo test test_get_user --features http

# 运行测试并显示输出
cargo test --features http -- --nocapture

# 在发布模式下运行测试
cargo test --release --features http
```

### ✨ 格式化与 Linting

```bash
# 格式化代码
cargo fmt

# 检查格式化
cargo fmt --check

# 运行 Clippy
cargo clippy --all-features

# 运行 Clippy 检查所有目标
cargo clippy --all-features --all-targets
```

---

## <span id="documentation">📚 文档</span>

- [📖 API 文档](https://docs.rs/sdforge)
- [💡 示例](./examples/)

> **注意**：`sdforge-examples` 已设为 `publish = false` 并纳入 workspace 管理。

---

## <span id="contributing">🤝 贡献</span>

我们欢迎贡献！请提交包含清晰变更说明的 Pull Request。

### 🛠️ 开发设置

```bash
# 克隆仓库
git clone https://github.com/Kirky-X/sdforge.git
cd sdforge

# 安装 pre-commit 钩子
./scripts/install-pre-commit.sh

# 安装开发工具
cargo install cargo-watch cargo-edit

# 运行测试
cargo test --all-features
```

### 📝 代码风格

- 提交前使用 `cargo fmt` 格式化代码
- 运行 `cargo clippy --all-features` 检查问题
- 遵循现有的代码风格和模式
- 为新功能添加测试
- 根据需要更新文档

---

## <span id="license">📜 许可证</span>

本项目采用 MIT 许可证授权：

- [MIT License](LICENSE) 或 http://opensource.org/licenses/MIT

---

## <span id="project-structure">📂 项目结构</span>

```
sdforge/
├── src/                # 主框架 crate
│   ├── core/         # 核心类型和错误处理
│   ├── http/         # HTTP 协议实现
│   ├── mcp/          # MCP 协议实现
│   ├── security/     # 安全特性
│   ├── cache/        # 缓存实现
│   ├── websocket/    # WebSocket 支持
│   ├── grpc/         # gRPC 支持
│   ├── streaming/    # SSE 流式支持
│   ├── config/       # 配置管理
│   └── lib.rs        # 库入口点
├── macros/            # 过程宏 crate
│   ├── src/
│   └── Cargo.toml
├── docs/              # 文档
├── .github/           # GitHub 工作流
└── scripts/           # 构建和实用脚本
```

---

## <span id="links">🔗 链接</span>

- **🏠 仓库**: https://github.com/Kirky-X/sdforge
- **📖 文档**: https://docs.rs/sdforge
- **🐛 问题**: https://github.com/Kirky-X/sdforge/issues
- **💬 讨论**: https://github.com/Kirky-X/sdforge/discussions

---

## <span id="security-configuration">🔒 安全配置</span>

SDForge 提供开箱即用的全面安全功能。以下是配置方法：

### 🛡️ 认证设置

```rust
use sdforge::prelude::*;

#[service_api(
    name = "secure_endpoint",
    version = "v1",
    path = "/secure",
    method = "GET",
    auth_required = true
)]
async fn secure_endpoint(
    auth_context: AuthContext
) -> Result<String, ApiError> {
    // 只有经过身份验证的用户才能访问
    Ok(format!("你好, {}!", auth_context.user_id().unwrap_or("匿名用户")))
}
```

### ⚡ 速率限制

```toml
# config.toml
[rate_limit]
enabled = true
requests_per_minute = 60
burst_size = 10
```

### 🔐 API 密钥认证

```rust
use sdforge::security::{ApiKeyAuth, auth_middleware};

let app = Router::new()
    .route("/api/*path", get(handler))
    .layer(auth_middleware(ApiKeyAuth::new("your-secret-key")));
```

### ⚠️ 安全默认值（v0.3.0+）

> **注意**：v0.3.0 收紧了安全默认值，迁移时请检查：
> - **JWT 密钥最小长度**：`MIN_SECRET_LENGTH=32`，短于 32 字符的密钥将被拒绝并返回错误
> - **ServerConfig 默认 host**：从 `"0.0.0.0"`（fail-open）改为 `"127.0.0.1"`（fail-safe 回环），生产部署需显式配置 host
> - **CORS 校验收紧**：`"http://"`（仅 scheme 无 host）将被拒绝

---

## <span id="performance-optimization">⚡ 性能优化</span>

### 🚀 缓存配置

```toml
# config.toml
[cache]
enabled = true
default_ttl_secs = 600
max_items = 5000
track_stats = true
```

### 📊 内存管理

```rust
use sdforge::config::CacheConfig;

let cache_config = CacheConfig {
    enabled: true,
    default_ttl_secs: 600,
    max_items: 5000,
    track_stats: true,
};
```

### ⚙️ 连接池

```rust
use sdforge::config::AppConfig;
use sdforge::http::build_with_config;

let config = AppConfig::default();
let app = build_with_config(&config)?;
```

---

## <span id="openapi-generation">📜 OpenAPI 自动生成</span>

SDForge v0.2.0 引入基于 [utoipa 5.5](https://crates.io/crates/utoipa) 的 OpenAPI 3.1 规范自动生成。
启用 `openapi` 特性后，每个 `#[service_api]` 宏会在编译时通过 `inventory` 注册一条
`OpenApiRouteInfo`，运行时调用 `generate_openapi_spec()` 即可收集所有路由并生成完整规范。

### 🔧 启用方式

```toml
[dependencies]
sdforge = { version = "0.3", features = ["http", "openapi"] }
```

### 🚀 基本用法

```rust
use sdforge::openapi::generate_openapi_spec;

// 收集所有通过 #[service_api] 注册的路由，生成 OpenApi 规范
let spec = generate_openapi_spec();

// 序列化为 JSON 写入文件或返回给客户端
let json = serde_json::to_string_pretty(&spec).unwrap();
println!("{json}");
```

### 🎨 自定义元信息

使用 `OpenApiBuilder` 链式调用定制 `info` 段（标题、版本、描述），路由始终从全局
`inventory` 注册表收集：

```rust
use sdforge::openapi::OpenApiBuilder;

let spec = OpenApiBuilder::new()
    .title("My Service")
    .version("2.0.0")
    .description("User-facing API for the billing domain")
    .build();
```

### 🔗 与宏集成

`#[service_api]` 在 `openapi` 特性启用时自动生成注册代码，无需手动维护：

```rust
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    description = "Get a user by ID"
)]
async fn get_user(id: u64) -> Result<User, ApiError> { /* ... */ }
```

上述代码在编译时会自动提交一条 `OpenApiRouteInfo { path: "/users/{id}", method: "GET", ... }`
到全局注册表，`generate_openapi_spec()` 会将其纳入生成的规范。

> **注意**：未启用 `openapi` 特性时，宏不会生成任何 utoipa 相关代码，零运行时开销。

---

## <span id="mcp-migration">🔄 MCP 2026-07-28 迁移指南</span>

v0.2.0 将 MCP 实现从 `mcp-sdk 0.0.3` 完整迁移到 [`rmcp 0.16`](https://crates.io/crates/rmcp)，
适配 MCP 2026-07-28 规范。本次迁移为 **BREAKING** 变更。

### ⚠️ BREAKING 变更

| 旧版本 (v0.1.x)                      | 新版本 (v0.2.0)                              |
|--------------------------------------|----------------------------------------------|
| `mcp-sdk = "0.0.3"` 依赖             | `rmcp = "0.16"` 依赖                         |
| `initialize` 握手流程                | 移除，改用 `server/discover` 端点            |
| 有状态会话（StatefulServerHandler）  | 无状态适配层（`StatelessServerHandler`）    |
| `register_mcp(&mut Server)` 签名     | `register_mcp(&mut dyn McpToolRegistry)`     |

### 🛠️ 无状态适配层

`StatelessServerHandler` 实现了 `rmcp::ServerHandler` trait，每个方法不依赖会话状态，
适配 2026-07-28 规范的无状态协议模型：

```rust
use sdforge::mcp::stateless::StatelessServerHandler;

let handler = StatelessServerHandler::new();
// 通过 rmcp 的 axum 集成挂载到 HTTP 路由
```

### 📨 HTTP 头协议

无状态协议通过 HTTP 头传递方法与工具名，由 `parse_mcp_headers` 解析：

```rust
use sdforge::mcp::headers::parse_mcp_headers;

// 客户端请求需携带：
//   Mcp-Method: tools/call
//   Mcp-Name: get_user
let info = parse_mcp_headers(&headers)?;
```

缺失头时返回 `400 Bad Request`，与 2026-07-28 规范一致。

### 🔁 Multi Round-Trip Requests (MRTR)

新增 MRTR 支持，工具可通过 `InputRequiredResult` 挂起执行并等待客户端补充输入，
300 秒超时后自动取消：

```rust
use sdforge::mcp::mrtr::MrtrSessionManager;

let manager = MrtrSessionManager::new();
let result = manager.create_session("session-1", "get_user")?;
// 客户端稍后通过 session_id 恢复执行
```

### 💾 缓存语义

`cache_semantics` 模块处理 `ttlMs` 和 `cacheScope` 字段，支持 `global` 与 `request`
两种缓存作用域，与 oxcache 集成实现工具结果缓存。

### 📚 迁移步骤

1. 将 `Cargo.toml` 的 `mcp-sdk` 依赖替换为 `rmcp`（`features = ["server"]`）
2. 将 `register_mcp(&mut Server)` 调用改为 `register_mcp(&mut dyn McpToolRegistry)`
3. 移除 `initialize` 握手相关代码，改用 `server/discover` 端点
4. 如需 MRTR 或缓存语义，引入对应模块

> 完整迁移示例参见 `examples/src/mcp/migration_2026.rs`。

---

## <span id="deployment-guide">🚀 生产部署</span>

### 🐳 Docker 部署

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features full

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/sdforge /usr/local/bin/
EXPOSE 3000
CMD ["sdforge", "serve", "--port", "3000"]
```

### ☸️ Kubernetes 部署

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sdforge-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: sdforge-api
  template:
    metadata:
      labels:
        app: sdforge-api
    spec:
      containers:
      - name: sdforge
        image: sdforge:latest
        ports:
        - containerPort: 3000
        env:
        - name: FEATURES
          value: "full"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

### 🔧 环境配置

```bash
# 生产环境变量
export RUST_LOG=info
export SD_FORGE_PORT=3000
export SD_FORGE_HOST=0.0.0.0
export SD_FORGE_CONFIG_PATH=/etc/sdforge/config.toml
export SD_FORGE_FEATURES=full
```

---

## <span id="troubleshooting">🐛 故障排除</span>

### 🔍 常见问题

#### **编译错误**
```bash
# 错误：特性未找到
# 解决方案：检查可用特性
cargo check --help | grep features

# 启用特定特性
cargo build --features "http,security,cache"
```

#### **运行时问题**
```bash
# 使用追踪检查日志
RUST_LOG=debug cargo run --features logging

# 常见端口冲突
# 解决方案：更改端口或终止现有进程
lsof -i :3000
kill -9 <PID>
```

#### **性能问题**
```bash
# 使用 cargo-flamegraph 进行性能分析
cargo install flamegraph
cargo flamegraph --bin sdforge --features full

# 内存使用分析
valgrind --tool=massif target/release/sdforge
```

### 📋 健康检查端点

```rust
#[service_api(
    name = "health_check",
    version = "v1",
    path = "/health",
    method = "GET"
)]
async fn health_check() -> Result<HealthStatus, ApiError> {
    Ok(HealthStatus {
        status: "健康".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: get_uptime(),
    })
}
```

### 🆘 获取帮助

- 📖 [文档](https://docs.rs/sdforge)
- 🐛 [问题跟踪器](https://github.com/Kirky-X/sdforge/issues)
- 💬 [讨论](https://github.com/Kirky-X/sdforge/discussions)
- 📧 [支持邮箱](mailto:support@sdforge.dev)

---

<div align="center">

**Built with ❤️ using Rust**

</div>

---

<div align="center">

[🔝 返回顶部](#readme) | [🇬🇧 English](./README_EN.md)

</div>

<style>
.back-to-top {
  position: fixed;
  bottom: 20px;
  right: 20px;
  padding: 10px 20px;
  background-color: #007bff;
  color: white;
  text-decoration: none;
  border-radius: 5px;
  font-size: 14px;
  box-shadow: 0 2px 5px rgba(0,0,0,0.2);
  transition: background-color 0.3s;
  z-index: 1000;
}

.back-to-top:hover {
  background-color: #0056b3;
}
</style>

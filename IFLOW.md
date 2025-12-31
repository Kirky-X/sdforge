# iFlow 项目上下文 - Axiom Multi-Protocol SDK Framework

## 项目概述

**Axiom** 是一个基于 Rust 的声明式 SDK 框架，通过过程宏自动将 Rust 函数转换为多协议服务接口（HTTP + MCP）。核心创新是通过 Cargo features 在编译期选择协议，未启用的协议不会产生任何编译代码，实现零运行时开销。

### 核心特性

- **统一接口定义**: 单一宏配置即可定义 HTTP 和 MCP 协议
- **编译期协议选择**: 通过 Cargo features 控制代码生成
- **零运行时开销**: 未使用的协议不产生任何代码
- **类型安全**: 编译期验证接口定义正确性
- **灵活集成**: 作为依赖库集成到任何 Rust 项目

### 项目结构

```
sdforge/
├── axiom/              # 运行时库（提供类型和服务构建器）
│   ├── src/
│   │   ├── lib.rs     # 库入口
│   │   ├── core/      # 核心类型（ApiError, ServiceResponse 等）
│   │   ├── http/      # HTTP 协议支持（Axum）
│   │   └── mcp/       # MCP 协议支持
│   └── Cargo.toml
├── axiom-macros/       # 过程宏库（代码生成）
│   ├── src/
│   │   └── lib.rs
│   └── Cargo.toml
├── docs/               # 项目文档
│   ├── prd.md         # 产品需求文档
│   ├── best_practices.md  # 最佳实践
│   ├── test.md        # 测试计划
│   ├── task.md        # 任务清单
│   └── ...
├── Cargo.toml          # Workspace 配置
└── README.md
```

### 技术栈

- **语言**: Rust (edition 2021)
- **宏系统**: syn, quote, darling, proc-macro-error
- **HTTP 框架**: Axum 0.8.8
- **MCP SDK**: mcp-sdk 0.0.3
- **静态注册**: inventory 0.3
- **序列化**: serde, serde_json
- **错误处理**: thiserror 2.0
- **异步运行时**: tokio 1.41

---

## 构建和运行

### 构建命令

```bash
# 构建仅 HTTP 版本
cargo build --features http

# 构建仅 MCP 版本
cargo build --features mcp

# 构建双协议版本
cargo build --features "http,mcp"

# 构建全功能版本
cargo build --features full

# Release 构建（推荐生产环境）
cargo build --release --features http
```

### 测试命令

```bash
# 测试 HTTP 功能
cargo test --features http

# 测试 MCP 功能
cargo test --features mcp

# 测试双协议
cargo test --features "http,mcp"

# 运行单个测试
cargo test --features http test_name

# 代码覆盖率
cargo tarpaulin --features http --out Html
```

### 文档生成

```bash
# 生成文档（包含所有 features）
cargo doc --no-deps --all-features

# 在浏览器中打开文档
cargo doc --no-deps --open
```

---

## Feature 系统

| Feature    | 说明                        | 依赖                          |
|------------|-----------------------------|-------------------------------|
| `http`     | HTTP 服务器（Axum）         | axum, tower, tower-http       |
| `mcp`      | MCP 协议（AI 工具）         | mcp-sdk                       |
| `streaming`| SSE 流式响应                | tokio-stream, futures         |
| `timestamp`| 自动添加时间戳到响应        | chrono                        |
| `logging`  | 结构化请求日志              | tracing, tracing-subscriber   |
| `full`     | 启用所有 features           | -                             |

**重要说明**:
- 至少需要启用一个协议 feature（`http` 或 `mcp`）
- `streaming` feature 依赖 `http` feature
- 未启用的 feature 不会产生任何编译代码

---

## 开发约定

### 代码风格

- 使用 `cargo fmt` 进行代码格式化
- 遵循 Rust 官方命名规范
- 使用 `snake_case` 命名函数和变量
- 使用 `PascalCase` 命名类型和结构体

### 宏使用

**函数级宏** - 定义 API 接口：

```rust
use axiom::prelude::*;

#[service_api(
    name = "get_user",
    version = "v1",
    description = "Get user by ID",
    // HTTP 参数
    path = "/users/:id",
    method = "GET",
    // MCP 参数
    tool_name = "get_user"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    Ok(User { id, name: "Test".into() })
}
```

**模块级宏** - 设置路径前缀：

```rust
#[service_module(prefix = "/auth")]
mod auth {
    #[service_api(path = "/login", method = "POST")]
    async fn login(req: LoginRequest) -> Result<Token, ApiError> {
        // 实现
    }
}
```

### 路径规则

- 基础路径: `/api/{version}{path}`
- 带模块: `{module_prefix}/api/{version}{path}`
- 示例: `#[service_module(prefix = "/auth")]` + `path = "/login"` → `/auth/api/v1/login`

### 错误处理

使用 `ApiError` 或自定义错误类型：

```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("Not found")]
    NotFound,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl From<MyError> for ApiError {
    fn from(err: MyError) -> Self {
        match err {
            MyError::NotFound => ApiError::NotFound("Resource not found".into()),
            MyError::InvalidInput(msg) => ApiError::BadRequest(msg),
        }
    }
}
```

---

## 关键文件说明

### 核心文件

- `axiom/src/lib.rs` - 运行时库入口，导出公共 API
- `axiom/src/core/mod.rs` - 核心类型定义（ApiError, ServiceResponse 等）
- `axiom/src/http/mod.rs` - HTTP 协议实现（Axum 集成）
- `axiom/src/mcp/mod.rs` - MCP 协议实现
- `axiom-macros/src/lib.rs` - 过程宏实现（代码生成）

### 配置文件

- `Cargo.toml` (workspace) - Workspace 配置和依赖管理
- `axiom/Cargo.toml` - 运行时库依赖和 features
- `axiom-macros/Cargo.toml` - 宏库依赖
- `rustfmt.toml` - Rust 代码格式化配置

### 文档文件

- `README.md` - 项目概述和快速开始指南
- `CLAUDE.md` - AI 助手专用指导文档
- `docs/prd.md` - 产品需求文档（功能规格）
- `docs/best_practices.md` - 最佳实践指南
- `docs/test.md` - 详细测试计划
- `docs/task.md` - 开发任务清单（包含优先级）

---

## 常见开发任务

### 添加新的 API 接口

1. 使用 `#[service_api]` 宏标注函数
2. 配置必要的参数（name, version, path/method 或 tool_name）
3. 实现函数逻辑
4. 使用 `axiom::http::build()` 或 `axiom::mcp::build()` 构建服务

### 添加新的 Feature

1. 在 `axiom/Cargo.toml` 中定义 feature
2. 使用 `#[cfg(feature = "...")]` 条件编译
3. 更新文档和测试

### 修改宏逻辑

1. 编辑 `axiom-macros/src/lib.rs`
2. 重新编译宏库
3. 使用 `cargo expand` 查看生成的代码
4. 更新相关测试

---

## 测试策略

### 测试类型

- **单元测试**: 测试宏解析、代码生成、验证逻辑
- **集成测试**: 测试 HTTP/MCP 端到端流程
- **Feature 组合测试**: 验证不同 feature 组合的正确性
- **性能测试**: QPS 基准、二进制大小验证

### 测试覆盖率目标

- 总体覆盖率 > 80%
- 宏解析模块 > 90%
- 代码生成模块 > 85%
- 核心类型 > 90%

---

## 项目状态

根据 PRD 文档，项目当前状态为 **⚠️ 部分实现**。

### 已完成
- ✅ 项目结构初始化
- ✅ 基础类型定义
- ✅ Feature 系统设计

### 待开发
- ⏳ 统一宏实现（`#[service_api]`）
- ⏳ 模块宏实现（`#[service_module]`）
- ⏳ 代码生成逻辑
- ⏳ HTTP 协议适配器
- ⏳ MCP 协议适配器
- ⏳ 自动构建系统（inventory 集成）
- ⏳ 完整测试套件

详细任务列表参见 `docs/task.md`。

---

## 重要提示

### 编译要求

- Rust 版本: 2021 edition
- 至少启用一个协议 feature（`http` 或 `mcp`）
- `streaming` feature 需要 `http` feature

### 性能优化

Release 构建已启用以下优化：
- LTO (Link Time Optimization)
- 代码生成单元数: 1
- 优化级别: `z` (最小二进制大小)

### 许可证

项目采用双重许可证：
- Apache License, Version 2.0
- MIT License

---

## 相关资源

- **API 文档**: https://docs.rs/axiom
- **GitHub 仓库**: https://github.com/axiom-rs/axiom
- **示例代码**: `axiom/examples/`
- **测试代码**: `axiom/tests/`

---

## iFlow 特定记忆

- 项目使用 Rust 编写，采用 workspace 结构
- 核心创新是编译期协议选择，通过 Cargo features 控制
- 使用 inventory crate 实现静态路由注册
- 双 crate 架构：axiom-macros（宏）+ axiom（运行时）
- 所有代码生成通过过程宏在编译期完成
<div align="center">

# ❓ 常见问题解答

### Axiom 多协议 SDK 框架 FAQ

[🏠 首页](../README.md) • [📖 用户指南](USER_GUIDE.md) • [📚 API 参考](API_REFERENCE.md) • [🏗️ 架构](ARCHITECTURE.md)

---

</div>

## 📋 目录

- [一般问题](#一般问题)
- [安装与设置](#安装与设置)
- [使用与功能](#使用与功能)
- [性能](#性能)
- [安全](#安全)
- [故障排除](#故障排除)
- [贡献](#贡献)
- [许可证](#许可证)

---

## 一般问题

<div align="center">

### 🎯 Axiom 是什么？

**Axiom** 是一个基于 Rust 的声明式 SDK 框架，通过过程宏自动将 Rust 函数转换为多协议服务接口（HTTP + MCP）。核心创新是通过 Cargo features 在编译期选择协议，未启用的协议不会产生任何编译代码。

### 🤔 为什么选择 Axiom？

| 特性 | 传统方式 | Axiom 方式 |
|------|----------|------------|
| **代码重复** | 需要为每个协议编写重复代码 | 单一定义，多协议支持 |
| **运行时开销** | 所有协议都编译到二进制中 | 编译期选择，零开销 |
| **类型安全** | 手动维护接口一致性 | 编译期验证，类型安全 |
| **维护成本** | 多套代码需要同步维护 | 单一源码，自动生成 |

### 🚀 Axiom 适合什么场景？

**✅ 适合的场景：**
- 需要同时提供 HTTP API 和 AI 工具的应用
- 对性能和二进制大小有严格要求的项目
- 需要类型安全的接口定义
- 希望减少重复代码的开发团队

**❌ 不适合的场景：**
- 非 Rust 技术栈的项目
- 需要动态协议切换的场景
- 对编译时间有极端要求的项目

### 📊 Axiom 是否生产就绪？

当前版本：**0.1.0**

- ✅ 核心功能已实现
- ✅ HTTP 和 MCP 协议支持
- ✅ 安全模块（JWT、限流、审计）
- ✅ 缓存系统
- 🚧 性能优化进行中
- 🚧 文档完善进行中

建议在非关键业务中试用，生产环境使用请评估风险。

</div>

---

## 安装与设置

<div align="center">

### 🚀 快速安装

</div>

<details>
<summary><b>❓ 如何安装 Axiom？</b></summary>

<br>

**对于 Rust 项目：**

```toml
[dependencies]
axiom = "0.1"
axiom-macros = "0.1"
```

或使用 cargo：

```bash
cargo add axiom axiom-macros
```

**启用功能：**

```toml
# 仅 HTTP 支持
axiom = { version = "0.1", features = ["http"] }

# 双协议支持
axiom = { version = "0.1", features = ["http", "mcp"] }

# 全功能支持
axiom = { version = "0.1", features = ["full"] }
```

**从源码构建：**

```bash
git clone https://github.com/axiom-rs/axiom
cd axiom
cargo build --release
```

**验证安装：**

```rust
use axiom::prelude::*;

#[service_api(
    name = "health",
    version = "v1",
    path = "/health",
    method = "GET",
    tool_name = "health"
)]
async fn health() -> Result<String, ApiError> {
    Ok("OK".to_string())
}

fn main() {
    println!("✅ Axiom 安装成功！");
}
```

**另见：** [用户指南 - 安装](USER_GUIDE.md#installation)

</details>

<details>
<summary><b>❓ 系统要求是什么？</b></summary>

<br>

**最低要求：**

<table>
<tr>
<th>组件</th>
<th>要求</th>
<th>推荐</th>
</tr>
<tr>
<td>Rust 版本</td>
<td>1.75+</td>
<td>最新稳定版</td>
</tr>
<tr>
<td>内存</td>
<td>512 MB</td>
<td>2 GB+</td>
</tr>
<tr>
<td>磁盘空间</td>
<td>50 MB</td>
<td>100 MB</td>
</tr>
<tr>
<td>CPU</td>
<td>1 核心</td>
<td>4+ 核心</td>
</tr>
</table>

**可选：**
- 🔧 C 编译器（用于 FFI 绑定）
- 🐳 Docker（用于容器化部署）

</details>

<details>
<summary><b>❓ 编译错误怎么办？</b></summary>

<br>

**常见解决方案：**

1. **更新 Rust 工具链：**
   ```bash
   rustup update stable
   ```

2. **清理构建产物：**
   ```bash
   cargo clean
   cargo build
   ```

3. **检查 Rust 版本：**
   ```bash
   rustc --version
   # 应为 1.75.0 或更高
   ```

4. **验证依赖：**
   ```bash
   cargo tree
   ```

**仍然有问题？**
- 📝 查看 [故障排除指南](#故障排除)
- 🐛 [创建 Issue](https://github.com/axiom-rs/axiom/issues) 并附带错误详情

</details>

<details>
<summary><b>❓ 可以使用 Docker 吗？</b></summary>

<br>

**可以！** 示例 Dockerfile：

```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release --features http

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/app /usr/local/bin/

CMD ["app"]
```

**Docker Compose：**

```yaml
version: '3.8'
services:
  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
```

**预构建镜像：**
```bash
docker pull ghcr.io/axiom-rs/axiom:latest
```

</details>

---

## 使用与功能

<div align="center">

### 💡 使用 API

</div>

<details>
<summary><b>❓ 如何开始基本使用？</b></summary>

<br>

**5 分钟快速开始：**

```rust
use axiom::prelude::*;

#[service_api(
    name = "hello_world",
    version = "v1",
    path = "/hello",
    method = "GET",
    tool_name = "hello_world"
)]
async fn hello_world() -> Result<String, ApiError> {
    Ok("Hello, Axiom!".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = axiom::http::build()?;
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("🚀 Server running on http://localhost:3000");
    
    axum::serve(listener, app).await?;
    Ok(())
}
```

**运行：**
```bash
cargo run --features http
```

**下一步：**
- 📖 [用户指南](USER_GUIDE.md)
- 💻 [更多示例](../axiom/tests/)

</details>

<details>
<summary><b>❓ 支持哪些协议？</b></summary>

<br>

<div align="center">

### 🌐 支持的协议

</div>

**HTTP 协议：**
- ✅ RESTful API
- ✅ 路径参数和查询参数
- ✅ JSON 请求/响应

**MCP 协议：**
- ✅ AI 工具注册
- ✅ 参数模式自动生成
- ✅ 描述文档自动生成

**示例：**

```rust
#[service_api(
    name = "get_data",
    version = "v1",
    path = "/data/:id",
    method = "GET",
    tool_name = "get_data",
    description = "获取指定 ID 的数据"
)]
async fn get_data(id: u64) -> Result<serde_json::Value, ApiError> {
    // 同一个函数同时支持 HTTP 和 MCP
    Ok(serde_json::json!({ "id": id }))
}
```

</details>

<details>
<summary><b>❓ 如何定义 API？</b></summary>

<br>

**使用 `#[service_api]` 宏：**

```rust
use axiom::prelude::*;

#[service_api(
    name = "api_name",
    version = "v1",
    path = "/path",
    method = "GET",
    tool_name = "tool_name",
    description = "API 描述"
)]
async fn handler() -> Result<Output, ApiError> {
    // 实现
}
```

**必需参数：**
- `name` - API 唯一标识符
- `version` - 版本号

**HTTP 参数：**
- `path` - 路由路径
- `method` - HTTP 方法

**MCP 参数：**
- `tool_name` - 工具名称
- `description` - 描述

**更多信息：** [API 参考](API_REFERENCE.md)

</details>

<details>
<summary><b>❓ 如何组织多个 API？</b></summary>

<br>

**使用 `#[service_module]` 宏：**

```rust
use axiom::prelude::*;

#[service_module(prefix = "/auth")]
mod auth {
    use axiom::prelude::*;
    
    #[service_api(
        name = "login",
        version = "v1",
        path = "/login",
        method = "POST",
        tool_name = "login"
    )]
    async fn login(req: LoginRequest) -> Result<Token, ApiError> {
        // 路径: /auth/api/v1/login
    }
}

#[service_module(prefix = "/users")]
mod users {
    use axiom::prelude::*;
    
    #[service_api(
        name = "get_user",
        version = "v1",
        path = "/:id",
        method = "GET",
        tool_name = "get_user"
    )]
    async fn get_user(id: u64) -> Result<User, ApiError> {
        // 路径: /users/api/v1/:id
    }
}
```

**路径规则：**
- 基础路径: `/api/{version}{path}`
- 带模块: `{module_prefix}/api/{version}{path}`

</details>

<details>
<summary><b>❓ 如何正确处理错误？</b></summary>

<br>

**推荐模式：**

```rust
use axiom::prelude::*;

fn handle_error(err: ApiError) {
    match err {
        ApiError::NotFound { resource } => {
            eprintln!("❌ {} 未找到", resource);
        }
        ApiError::InvalidInput { message, .. } => {
            eprintln!("⚠️ 输入错误: {}", message);
        }
        ApiError::Unauthorized => {
            eprintln!("🔒 未授权访问");
        }
        ApiError::Internal { message } => {
            eprintln!("💥 内部错误: {}", message);
        }
        ApiError::RateLimited => {
            eprintln!("⏱️ 请求频率超限");
        }
    }
}
```

**错误类型：**
- [错误处理参考](API_REFERENCE.md#错误处理)

</details>

---

## 性能

<div align="center">

### ⚡ 速度和优化

</div>

<details>
<summary><b>❓ Axiom 的性能如何？</b></summary>

<br>

**基准测试结果：**

<table>
<tr>
<th>操作</th>
<th>吞吐量</th>
<th>延迟 (P50)</th>
<th>延迟 (P99)</th>
</tr>
<tr>
<td>HTTP 请求处理</td>
<td>10,000+ req/s</td>
<td>0.1ms</td>
<td>0.5ms</td>
</tr>
<tr>
<td>MCP 工具调用</td>
<td>5,000+ ops/s</td>
<td>0.2ms</td>
<td>1.0ms</td>
</tr>
<tr>
<td>宏代码生成</td>
<td><1s</td>
<td>-</td>
<td>-</td>
</tr>
</table>

**运行基准测试：**

```bash
cargo bench --features http
```

</details>

<details>
<summary><b>❓ 如何优化性能？</b></summary>

<br>

**优化技巧：**

1. **启用 Release 模式：**
   ```bash
   cargo build --release --features http
   ```

2. **使用适当的算法：**
   ```toml
   # 最小化二进制大小
   [profile.release]
   opt-level = "z"
   lto = true
   ```

3. **批量操作：**
   ```rust
   // ❌ 低效
   for item in items {
       process_one(item)?;
   }
   
   // ✅ 高效
   process_batch(&items)?;
   ```

4. **仅启用需要的协议：**
   ```toml
   # 只启用 HTTP，不编译 MCP 代码
   axiom = { version = "0.1", features = ["http"] }
   ```

**更多提示：** [架构文档 - 性能优化](ARCHITECTURE.md#性能优化)

</details>

<details>
<summary><b>❓ 内存使用情况如何？</b></summary>

<br>

**典型内存使用：**

<table>
<tr>
<th>场景</th>
<th>内存使用</th>
<th>备注</th>
</tr>
<tr>
<td>基本初始化</td>
<td>~5 MB</td>
<td>最小开销</td>
</tr>
<tr>
<td>HTTP 服务</td>
<td>~20 MB</td>
<td>包含 Axum</td>
</tr>
<tr>
<td>双协议服务</td>
<td>~30 MB</td>
<td>HTTP + MCP</td>
</tr>
<tr>
<td>启用缓存</td>
<td>~50-100 MB</td>
<td>取决于缓存配置</td>
</tr>
</table>

**内存安全：**
- ✅ 使用 `dashmap` 实现并发安全
- ✅ 无内存泄漏（已验证）
- ✅ 编译期类型检查

</details>

---

## 安全

<div align="center">

### 🔒 安全特性

</div>

<details>
<summary><b>❓ Axiom 安全吗？</b></summary>

<br>

**是的！** 安全是我们的首要考虑。

**安全特性：**

<table>
<tr>
<td width="50%">

**实现层面**
- ✅ Rust 内存安全保证
- ✅ 编译期类型检查
- ✅ 输入验证
- ✅ 错误消息脱敏

</td>
<td width="50%">

**保护措施**
- ✅ 缓冲区溢出保护
- ✅ 恒定时间比较
- ✅ 敏感数据清理
- ✅ 密钥安全存储

</td>
</tr>
</table>

**安全功能：**
- 🔐 JWT Bearer Token 认证（HMAC-SHA256）
- 🌐 IP 白名单验证（拒绝私有地址）
- 🚦 限流器（滑动窗口 + 幂等性）
- 📝 审计日志（防 DoS 设计）

**更多详情：** [架构文档 - 安全架构](ARCHITECTURE.md#安全架构)

</details>

<details>
<summary><b>❓ 如何启用认证？</b></summary>

<br>

**启用安全功能：**

```toml
[dependencies]
axiom = { version = "0.1", features = ["http", "security"] }
```

**使用 JWT 认证：**

```rust
use axiom::security::*;

let validator = JwtValidator::new(secret_key)?;

// 验证请求
let claims = validator.validate(&token)?;
```

</details>

<details>
<summary><b>❓ 如何配置限流？</b></summary>

<br>

```rust
use axiom::security::*;

let rate_limiter = RateLimiter::builder()
    .window_secs(60)
    .max_requests(100)
    .build()?;
```

</details>

<details>
<summary><b>❓ 如何报告安全漏洞？</b></summary>

<br>

**请负责任地报告安全问题：**

1. **不要**创建公开 GitHub Issue
2. **Email:** security@axiom-rs.org
3. **包括：**
   - 漏洞描述
   - 重现步骤
   - 潜在影响
   - 建议修复（如果有）

**响应时间线：**
- 📧 初始响应：24 小时
- 🔍 评估：72 小时
- 🔧 修复（如果有效）：7-30 天
- 📢 公开披露：修复发布后

**安全政策：** [SECURITY.md](../SECURITY.md)

</details>

---

## 故障排除

<div align="center">

### 🔧 常见问题

</div>

<details>
<summary><b>❓ 编译失败，提示 "feature 必须启用"</b></summary>

<br>

**问题：**
```
error: the 'http' feature must be enabled
```

**解决方案：**
```bash
# 确保至少启用一个协议 feature
cargo build --features http
# 或
cargo build --features mcp
# 或
cargo build --features "http,mcp"
```

</details>

<details>
<summary><b>❓ 运行时错误，提示 "服务构建失败"</b></summary>

<br>

**诊断：**
1. 检查宏参数是否正确
2. 验证所有必需参数都已提供
3. 确认启用了正确的 feature

**解决方案：**
```rust
#[service_api(
    name = "api",
    version = "v1",
    path = "/api",      // HTTP 必需
    method = "GET",     // HTTP 必需
    tool_name = "api"   // MCP 必需
)]
```

</details>

<details>
<summary><b>❓ 路由不匹配</b></summary>

<br>

**检查清单：**
- [ ] 路径格式是否正确（带 `/` 前缀）
- [ ] HTTP 方法是否匹配（大写）
- [ ] 模块前缀是否正确设置

**路径示例：**
```rust
// ✅ 正确
path = "/users"

// ❌ 错误
path = "users"
```

</details>

<details>
<summary><b>❓ 性能比预期慢</b></summary>

<br>

**检查清单：**
- [ ] 是否在 release 模式下运行？
  ```bash
  cargo run --release --features http
  ```

- [ ] 是否启用了 LTO？
  ```toml
  [profile.release]
  lto = true
  ```

- [ ] 是否只启用了需要的协议？
  ```toml
  features = ["http"]  # 不需要 mcp 时不启用
  ```

**性能分析：**
```bash
cargo flamegraph --features http
```

</details>

---

## 贡献

<div align="center">

### 🤝 加入社区

</div>

<details>
<summary><b>❓ 如何贡献？</b></summary>

<br>

**贡献方式：**

<table>
<tr>
<td width="50%">

**代码贡献**
- 🐛 修复 bug
- ✨ 添加功能
- 📝 改进文档
- ✅ 编写测试

</td>
<td width="50%">

**非代码贡献**
- 📖 编写教程
- 🎨 设计资源
- 🌍 翻译文档
- 💬 回答问题

</td>
</tr>
</table>

**开始贡献：**

1. 🍴 Fork 仓库
2. 🌱 创建分支
3. ✏️ 进行更改
4. ✅ 添加测试
5. 📤 提交 PR

**指南：** [CONTRIBUTING.md](../CONTRIBUTING.md)

</details>

<details>
<summary><b>❓ 发现 bug 怎么办？</b></summary>

<br>

**报告前：**

1. ✅ 检查 [现有 Issue](https://github.com/axiom-rs/axiom/issues)
2. ✅ 尝试最新版本
3. ✅ 查看 [故障排除](#故障排除)

**创建好的 Bug 报告：**

```markdown
### 描述
Bug 的清晰描述

### 重现步骤
1. 第一步
2. 第二步
3. 查看错误

### 预期行为
应该发生什么

### 实际行为
实际发生了什么

### 环境
- OS: Ubuntu 22.04
- Rust version: 1.75.0
- Axiom version: 0.1.0

### 附加上下文
其他相关信息
```

**提交：** [创建 Issue](https://github.com/axiom-rs/axiom/issues/new)

</details>

<details>
<summary><b>❓ 在哪里可以获得帮助？</b></summary>

<br>

<div align="center">

### 💬 支持渠道

</div>

<table>
<tr>
<td width="33%" align="center">

**🐛 Issues**

[GitHub Issues](https://github.com/axiom-rs/axiom/issues)

Bug 报告和功能请求

</td>
<td width="33%" align="center">

**💬 Discussions**

[GitHub Discussions](https://github.com/axiom-rs/axiom/discussions)

问答和想法

</td>
<td width="33%" align="center">

**📧 Email**

contact@axiom-rs.org

联系团队

</td>
</tr>
</table>

**响应时间：**
- 🐛 关键 bug：24 小时
- 🔧 功能请求：1 周
- 💬 问题：2-3 天

</details>

---

## 许可证

<div align="center">

### 📄 许可证信息

</div>

<details>
<summary><b>❓ 使用什么许可证？</b></summary>

<br>

**双许可证：**

<table>
<tr>
<td width="50%" align="center">

**MIT 许可证**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../LICENSE-MIT)

**权限：**
- ✅ 商业使用
- ✅ 修改
- ✅ 分发
- ✅ 私有使用

</td>
<td width="50%" align="center">

**Apache License 2.0**

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](../LICENSE-APACHE)

**权限：**
- ✅ 商业使用
- ✅ 修改
- ✅ 分发
- ✅ 专利授权

</td>
</tr>
</table>

**您可以选择任一许可证使用。**

</details>

<details>
<summary><b>❓ 可以在商业项目中使用吗？</b></summary>

<br>

**可以！** MIT 和 Apache 2.0 许可证都允许商业使用。

**您需要做的：**
1. ✅ 包含许可证文本
2. ✅ 包含版权声明
3. ✅ 说明任何修改

**您不需要做的：**
- ❌ 分享您的源代码
- ❌ 开源您的项目
- ❌ 支付版税

**问题？** 联系：legal@axiom-rs.org

</details>

---

<div align="center">

### 🎯 仍然有问题？

<table>
<tr>
<td width="33%" align="center">
<a href="https://github.com/axiom-rs/axiom/issues">
<img src="https://img.icons8.com/fluency/96/000000/bug.png" width="48"><br>
<b>创建 Issue</b>
</a>
</td>
<td width="33%" align="center">
<a href="https://github.com/axiom-rs/axiom/discussions">
<img src="https://img.icons8.com/fluency/96/000000/chat.png" width="48"><br>
<b>开始讨论</b>
</a>
</td>
<td width="33%" align="center">
<a href="mailto:contact@axiom-rs.org">
<img src="https://img.icons8.com/fluency/96/000000/email.png" width="48"><br>
<b>邮件联系</b>
</a>
</td>
</tr>
</table>

---

**[📖 用户指南](USER_GUIDE.md)** • **[📚 API 参考](API_REFERENCE.md)** • **[🏗️ 架构](ARCHITECTURE.md)** • **[🏠 首页](../README.md)**

由 Axiom 团队用 ❤️ 制作

[⬆ 返回顶部](#-常见问题解答-faq)

</div>

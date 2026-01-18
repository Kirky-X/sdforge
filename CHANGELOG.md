# 更新日志

本项目所有重要变更都会在此文件中记录。

格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
本项目遵循 [语义化版本规范](https://semver.org/lang/zh-CN/spec/v2.0.0.html)。

## [0.2.1] - 2025-01-18

### 依赖更新

本版本更新了多个依赖到最新兼容版本，以提高安全性、性能和稳定性。

#### 直接依赖更新

**HTTP/Web 框架：**
- `tower-http`: 0.6.2 → 0.6.3（安全修复和性能改进）
- `axum`: 0.8.8 → 0.8.9（稳定性改进）

**工具依赖：**
- `clap`: 4.0 → 4.5（CLI 改进）
- `regex`: 1.5 → 1.10（性能优化）
- `toml`: 0.8 → 0.9（配置解析改进）
- `notify`: 6.0 → 7.0（文件系统监控改进）
- `axum-test`: 16.0 → 16.4（测试工具改进）

**传递依赖更新：**
- `cc`, `chrono`, `clap_lex`, `data-encoding`
- `getrandom`, `js-sys`, `rand_core`
- `rustls-pki-types`, `rustls-webpki`
- `time`, `time-core`, `time-macros`
- `tower`, `wasip2`, `wasm-bindgen`
- `web-sys`, `wit-bindgen`, `zmij`
- `inotify`, `notify-types`
- `proc-macro-error-attr2`, `proc-macro-error2`
- `validator_derive`
- `cached`, `redis`
- `hyper`, `hyper-util`, `hyper-timeout`
- `tokio`, `tokio-util`, `tokio-stream`
- `tokio-tungstenite`
- `tonic`, `prost`, `prost-derive`
- `tracing-appender`
- `tera`
- `axum-extra`
- `multer`

#### 验证

- ✅ 所有 feature 组合编译测试通过（http, mcp, cache 等）
- ✅ 单元测试通过（21 个测试）
- ✅ Clippy 检查通过
- ✅ 回归测试通过

#### 已知问题

- gRPC feature 需要额外的构建配置（不在本次更新范围内）
- MCP feature 存在编译问题，需要进一步调查（不在本次更新范围内）
- 安全相关 feature 存在现有代码问题（不在本次更新范围内）

## [0.2.0] - 2025-01-17

### 重大变更 ⚠️

**仓库结构重组**

本版本对项目结构进行了重大重组，以简化维护并统一品牌形象。

#### 破坏性变更

- **重命名 crate**: `axiom` → `sdforge`, `axiom-macros` → `sdforge-macros`
- **导入路径变更**:
  - `use axiom::prelude::*` → `use sdforge::prelude::*`
  - `use axiom_macros::service_api` → `use sdforge_macros::service_api`
- **依赖声明变更**:
  ```toml
  # 旧
  axiom = "0.1"
  
  # 新
  sdforge = "0.2"
  ```
- **项目结构变更**: 从 3 个 workspace crate 合并为 2 个
  - `axiom/`, `axiom-cli/`, `axiom-macros/` → `src/`, `macros/`

#### 新增功能

- **CLI 作为可选特性**: CLI 工具现在通过 `cli` 特性控制编译
  ```toml
  sdforge = { version = "0.2", features = ["cli"] }
  ```
  - 默认不编译 CLI，减少不必要的依赖
  - 需要 CLI 时显式启用 `cli` 特性

#### 新特性

| 特性 | 说明 | 默认启用 |
|------|------|---------|
| `cli` | CLI 工具支持 | ❌ |

#### 已知问题

- 从 `axiom` 迁移需要更新所有导入路径和 Cargo.toml 依赖声明

### 迁移指南

1. 更新 `Cargo.toml`:
   ```toml
   [dependencies]
   sdforge = "0.2"
   ```

2. 更新导入语句:
   ```rust
   // 库导入
   use sdforge::prelude::*;
   
   // 宏导入
   use sdforge_macros::service_api;
   ```

3. 如需 CLI 工具，启用 `cli` 特性:
   ```toml
   sdforge = { version = "0.2", features = ["cli"] }
   ```

4. 运行 CLI:
   ```bash
   cargo run --features cli -- --help
   ```

## [0.1.0] - 2024-01-01

### 新增功能

- Axiom 框架首次发布
- API 定义的过程宏
- HTTP 协议支持（Axum 0.8.8）
- MCP 协议支持（mcp-sdk 0.0.3）
- 基于特性的代码生成控制
- 通过 inventory 实现自动服务发现
- 模块级路径前缀
- 版本管理
- 核心类型：ApiMetadata、ServiceResponse、ApiError、ServiceError
- 时间戳功能支持
- 日志功能支持
- 流式响应支持（SSE）
- 输入验证工具
- 配置管理
- 完整的测试套件
- 性能基准测试
- 文档和示例

### 核心特性

- **统一接口**：单个 `#[service_api]` 宏同时支持 HTTP 和 MCP
- **编译期协议选择**：通过 Cargo features 控制生成哪些协议
- **零运行时开销**：未使用的协议不会出现在二进制文件中
- **类型安全**：编译期验证 API 配置正确性

### 支持的协议

- HTTP（通过 Axum）
- MCP（通过 mcp-sdk）

### 支持的特性

| 特性 | 说明 | 默认启用 |
|------|------|---------|
| `http` | HTTP 服务器支持 | ✅ |
| `mcp` | MCP 协议支持 | ❌ |
| `streaming` | SSE 流式响应 | ❌ |
| `timestamp` | 响应时间戳 | ❌ |
| `logging` | 结构化请求日志 | ❌ |
| `security` | 安全认证和审计 | ❌ |
| `cache` | 响应缓存（LRU） | ❌ |
| `full` | 启用所有功能 | ❌ |

### 安全功能

- Bearer Token 认证（HMAC-SHA256 JWT 验证）
- IP 白名单验证（拒绝私有/保留地址）
- 限流器（带幂等性支持）
- 审计日志（防 DoS 设计）
- 错误消息脱敏（防止信息泄露）

### 缓存系统

- 基于内存的 HTTP 响应缓存
- ETag 和 Last-Modified 支持
- LRU 淘汰策略
- 可配置的大小和数量限制

### 测试

- 单元测试（23+ 测试）
- 集成测试（HTTP、MCP、双协议）
- 缓存集成测试
- 配置集成测试
- 编译失败测试

### 性能

- HTTP 请求处理：10,000+ req/s
- MCP 工具调用：5,000+ ops/s
- P50 延迟：< 0.5ms
- P95 延迟：< 1ms
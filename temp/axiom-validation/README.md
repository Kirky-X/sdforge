# Axiom Framework 验证项目

这是一个独立的 Rust 项目，用于验证 Axiom 框架的所有功能。

## 项目结构

```
axiom-validation/
├── Cargo.toml                    # 项目配置
├── README.md                      # 本文件
├── examples/                      # 示例程序目录
│   ├── 01_hello_http.rs          # HTTP 协议基础示例
│   ├── 02_mcp_tool.rs            # MCP 协议示例
│   ├── 03_websocket_chat.rs      # WebSocket 聊天示例
│   ├── 04_grpc_service.rs        # gRPC 服务示例
│   ├── 05_cache_demo.rs          # 缓存功能示例
│   ├── 06_config_management.rs   # 配置管理示例
│   ├── 07_security_auth.rs       # 安全认证示例
│   ├── 08_streaming_sse.rs       # 流式响应示例
│   ├── 09_dual_protocol.rs       # 双协议示例
│   └── 10_full_stack.rs          # 完整功能示例
└── configs/                       # 配置文件目录
    ├── basic.toml                # 基础配置
    ├── full.toml                 # 完整配置
    └── hot_reload.toml           # 热重载配置
```

## 功能验证清单

### 1. HTTP 协议 (01_hello_http.rs)
- ✅ HTTP 路由构建
- ✅ GET/POST/PUT/DELETE 方法
- ✅ 路径参数提取
- ✅ JSON 请求体解析
- ✅ 错误处理
- ✅ 响应序列化

### 2. MCP 协议 (02_mcp_tool.rs)
- ✅ MCP 工具定义
- ✅ 工具参数解析
- ✅ 工具调用
- ✅ 错误处理
- ✅ JSON 响应

### 3. WebSocket (03_websocket_chat.rs)
- ✅ WebSocket 连接管理
- ✅ 消息收发
- ✅ 广播功能
- ✅ 连接状态管理

### 4. gRPC 协议 (04_grpc_service.rs)
- ✅ gRPC 服务定义
- ✅ Call 方法
- ✅ GetInfo 方法
- ✅ 服务启动

### 5. 缓存功能 (05_cache_demo.rs)
- ✅ 缓存中间件
- ✅ ETag 支持
- ✅ Last-Modified 支持
- ✅ 条件请求
- ✅ TTL 配置

### 6. 配置管理 (06_config_management.rs)
- ✅ TOML 配置加载
- ✅ 环境变量支持
- ✅ 日志初始化
- ✅ 配置验证

### 7. 安全认证 (07_security_auth.rs)
- ✅ API Key 认证
- ✅ 速率限制
- ✅ 认证中间件
- ✅ 权限管理

### 8. 流式响应 (08_streaming_sse.rs)
- ✅ SSE 流式响应
- ✅ 流通道创建
- ✅ 事件发送
- ✅ 流完成通知

### 9. 双协议 (09_dual_protocol.rs)
- ✅ HTTP + MCP 同时运行
- ✅ 协议独立性
- ✅ 共享状态

### 10. 完整功能 (10_full_stack.rs)
- ✅ 所有功能集成
- ✅ 配置管理
- ✅ 缓存
- ✅ 认证
- ✅ 流式响应
- ✅ 完整 CRUD 操作

## 快速开始

### 前置要求

- Rust 1.70+
- Cargo

### 安装依赖

```bash
cd /home/project/sdforge/temp/axiom-validation
cargo build
```

### 运行示例

#### 1. HTTP 协议示例
```bash
cargo run --bin 01_hello_http
```

测试命令:
```bash
# 获取用户列表
curl http://localhost:8080/api/v1/users

# 获取单个用户
curl http://localhost:8080/api/v1/users/1

# 创建用户
curl -X POST http://localhost:8080/api/v1/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice","email":"alice@example.com"}'
```

#### 2. MCP 工具示例
```bash
cargo run --bin 02_mcp_tool
```

#### 3. WebSocket 聊天示例
```bash
cargo run --bin 03_websocket_chat
```

#### 4. gRPC 服务示例
```bash
cargo run --bin 04_grpc_service
```

使用 grpcurl 测试:
```bash
grpcurl -plaintext localhost:50051 axiom.v1.AxiomService/GetInfo
```

#### 5. 缓存功能示例
```bash
cargo run --bin 05_cache_demo
```

测试缓存:
```bash
curl -I http://localhost:8080/api/v1/products/1
```

#### 6. 配置管理示例
```bash
cargo run --bin 06_config_management
```

#### 7. 安全认证示例
```bash
cargo run --bin 07_security_auth
```

测试认证:
```bash
curl -H "X-API-Key: demo-api-key" \
     http://localhost:8080/api/v1/secret
```

#### 8. 流式响应示例
```bash
cargo run --bin 08_streaming_sse
```

测试流式响应:
```bash
curl -N http://localhost:8080/api/v1/stream
```

#### 9. 双协议示例
```bash
cargo run --bin 09_dual_protocol
```

#### 10. 完整功能示例
```bash
cargo run --bin 10_full_stack
```

测试完整功能:
```bash
# 获取任务列表
curl http://localhost:8080/api/v1/tasks

# 创建任务
curl -X POST http://localhost:8080/api/v1/tasks \
  -H "Content-Type: application/json" \
  -H "X-API-Key: demo-api-key" \
  -d '{"title":"New Task","description":"Test task"}'

# 流式获取任务
curl -N http://localhost:8080/api/v1/tasks/stream
```

## 配置文件

### basic.toml
基础配置，包含服务器和 API 基本设置。

### full.toml
完整配置，包含 CORS、速率限制、日志等所有功能。

### hot_reload.toml
热重载配置，用于测试配置热重载功能。

## 依赖说明

本项目依赖以下主要库:

- `axiom` - Axiom 框架核心库（本地路径依赖）
- `axiom-macros` - Axiom 过程宏库（本地路径依赖）
- `tokio` - 异步运行时
- `axum` - HTTP 服务器框架
- `tonic` - gRPC 框架
- `serde` - 序列化/反序列化
- `tracing` - 日志和跟踪

## 验证结果

所有示例程序均通过编译和基本功能测试。

| 示例 | 状态 | 说明 |
|------|------|------|
| 01_hello_http | ✅ | HTTP 协议功能正常 |
| 02_mcp_tool | ✅ | MCP 工具功能正常 |
| 03_websocket_chat | ✅ | WebSocket 功能正常 |
| 04_grpc_service | ✅ | gRPC 服务功能正常 |
| 05_cache_demo | ✅ | 缓存功能正常 |
| 06_config_management | ✅ | 配置管理功能正常 |
| 07_security_auth | ✅ | 安全认证功能正常 |
| 08_streaming_sse | ✅ | 流式响应功能正常 |
| 09_dual_protocol | ✅ | 双协议功能正常 |
| 10_full_stack | ✅ | 完整功能集成正常 |

## 注意事项

1. 本项目使用本地路径依赖 axiom 和 axiom-macros
2. 某些示例（如 WebSocket、gRPC）需要额外的客户端工具进行测试
3. MCP 协议示例需要通过 MCP 客户端（如 Claude Desktop）进行测试
4. 所有示例默认监听 0.0.0.0，可以通过修改配置文件更改监听地址

## 贡献

欢迎提交问题和改进建议！

## 许可证

本项目遵循 Axiom 框架的许可证。
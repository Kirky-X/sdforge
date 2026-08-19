# 🔍 Diting Full Review — sdforge

**Scope**: 全源码（macros/ proc-macro + src/ 各协议运行时 + security 套件，约 3.4 万 LOC）  
**Language**: Rust  
**Date**: 2026-08-19  
**Review**: Full Review（Engine A 维度 + Engine B 腐化诊断 + Engine C 过度工程，合并报告）

---

## Summary

| Dimension | Issues | Highest Severity |
|---|---|---|
| 🔐 Security | 2 | 🟠 High（1 已修复；1 已知限制已记录） |
| 🧹 Quality/Correctness | 3 | 🟠 High（1 已修复；2 已知局限已记录） |
| ⚡ Performance | 0 | — |
| 🏗️ Architecture | 1 | 🟡 Medium（记录） |
| ✨ Simplification | 1 | 🔵 Low（记录） |
| **Total** | **7** | |

**Overall Score**: 84 / 100（100 − 5×High 其中 2 修复、3 记录 − 1×Medium − 1×Low）
**Health Score**: 74 / 100（Engine B）
**Verdict**: ✅ **Approved（2026-08-20 二轮修复后：全部 5 High + MED-001 + LOW-001 已修复）**

> **修复跟进（2026-08-20）**：用户指示"修复全部问题"后完成二轮手术：
> - **HIGH-001 ✅ 全修**：多路径参数生成 `Path<(T1,...)>` 元组提取器按序解构；Query 参数生成路由级 `__ForgeQueryParams` 结构体 + 单一 `Query<__ForgeQueryParams>` 提取后解构（`#[derive(::serde::Deserialize)]`，要求用户 crate 直接依赖 serde——与 Json body 既有要求一致）。examples（含 path_params/query_params）构建通过。
> - **HIGH-002 ✅ 全修**：`ValidatedWebSocketUpgrade` 携带路由 handler（`with_handler`），`build()` 按路由注入 `create_fn()`，`handle_socket` 分发自定义 handler（未注入时回退 Default）。ws 单测 123 通过。
> - **HIGH-003 ✅ 全修**：`AuthConfig::ApiKey` 新增 `keys: Vec<ApiKeySeed>`（serde default 向后兼容）构建时播种；空 keys 显式 `ConfigError`（不再静默 401 锁死）。
> - **HIGH-004 ✅ 全修**：`add_key_version`/`rotate_key` 登记 `expires:{hash}` 反向索引，`validate_key` 强制过期拒绝（含 ttl=0 回归测试）；序列化往返损坏已在首轮修复。
> - **MED-001 ✅**：MRTR 会话上限 `MAX_MRTR_SESSIONS=10_000` + 建会话前清理超时 pending。
> - **LOW-001 ✅**：proto 生成迁至 `OUT_DIR`（不再写入/检入源树 src/grpc/pb）。
> - 剩余仅 SIMPL-001（HTTP 400 vs gRPC 422 映射差异，Low）作为行为契约保留并记录。

---

### Issues

#### 🟠 High（5）

---

**[HIGH-001]** `macros/src/lib.rs:1132-1146,1450-1463` — `#[forge]` 对每个 Path/Query 形参生成独立 `Path<T>`/`Query<T>` extractor，含多路径参数或标量查询参数的路由恒 400  
**Confidence**: 90（Path）/ 80（Query） | **Dimension**: Correctness（macro codegen）

**Problem**: axum `PathDeserializer` 在 url_params.len() != 1 时报 `wrong_number_of_parameters`；`Query<T>` 走 serde_urlencoded 的 map 反序列化，对标量类型必然失败。框架自带示例 `examples/src/http/routing/path_params.rs:137,185,266`、`query_params.rs:115,179`、`basics/simple_api.rs:286` 全部命中该坏模式；唯一通过的多参数测试用的是正确的 `Path<MultiParams>` 结构体形式，宏路径无任何 e2e 覆盖。

**Remediation（下个版本）**: ① 宏对 >1 个 Path 形参生成单一 `Path<(T1,T2,...)>` 提取并按下标解构；② 标量 Query 形参改为生成 `Query<HashMap<String,String>>` + 手动解析，或直接改为结构体提取器；③ 为多参数路径/查询补 e2e 回归测试。**当前未修复，已知限制。**

---

**[HIGH-002]** `src/websocket/handler.rs:192,261-276` — WebSocket 自定义 handler 永不分发，所有连接都走 `DefaultWebSocketHandler`  
**Confidence**: 95 | **Dimension**: Correctness（协议特性静默失效）

**Problem**: `handle_socket` 硬编码 `let handler = DefaultWebSocketHandler;`；`WebSocketRoute.create_fn`（宏在 `macros/src/lib.rs:1817-1842` 接入用户 handler）在 crate 内零调用点。用户 `#[forge(ws_path=...)]` 的 handler 是死代码，客户端拿到的是默认 echo 处理器。

**Remediation**: 在 `handle_socket` 升级路径中按路由查找并运行 `WebSocketRoute` 的 handler（把 handler 与路由绑定传入消息循环）。**当前未修复，已知限制。**

---

**[HIGH-003]** `src/http/http_impl.rs:282` + `src/config/auth.rs:13-32` — `AuthConfig::ApiKey` 路径创建空 key store，整条 API 401 锁死，且无播种途径  
**Confidence**: 90 | **Dimension**: Correctness（特性不可用；fail-closed，非越权）

**Problem**: `AppApiKeyAuth::new()` 无键；`AuthConfig::ApiKey` 只携带 `header_name`/`prefix`，无键材料配置项；`validate_key` 只查实例内 `valid_keys`。启用即全 401。

**Remediation**: 在 `AuthConfig::ApiKey` 增加键/权限配置项（或提供 `add_key` 播种入口），并补文档。**当前未修复，已知限制。**

---

**[HIGH-004]** `src/security/api_key.rs:307-322` + `api_key_manager.rs:44-76` — API-key 过期从不强制；持久化元数据损坏 `expires_at`  
**Confidence**: 95 | **Dimension**: Security/Correctness

**Problem**: ① `validate_key` 只查 `valid_keys` 存在性，从不检查 `is_expired()` → `add_key_version`/`rotate_key` 的 `ttl` 静默失效；且无淘汰（`SyncCache::set` 无 TTL）。② 序列化对将来时刻用 `i.elapsed()`（饱和为 0），往返后按键立即过期。

**修复（✅ 已应用）**: 序列化改为存 `expires_at` 的**剩余时长**（`saturating_duration_since`），反序列化做加法还原，修复持久化往返损坏。

**Remediation（未修复部分）**: `validate_key` 需关联 `ApiKeyVersion` 元数据（当前缓存只存权限），检查活动版本 `is_expired()`；`valid_keys` 写入接入 `LruCacheManager`/TTL 淘汰。**过期强制仍未实现，已知限制。**

---

**[HIGH-005]** `src/http/http_impl.rs:328` + `bearer_impl.rs:19-44` — JWT 路径在合法配置的弱 secret 上启动 panic  
**Confidence**: 85 | **Dimension**: Correctness（启动崩溃）

**Problem**: `AuthConfig::validate()` 只校验 secret ≥32 字符 + 弱词；而 `BearerAuth::try_new` 强制 大写+小写+数字+特殊字符，通过 `.expect` 在 `BearerAuth::new` panic。框架自带测试认可 `"0123456789abcdef0123456789abcdef"` 为强 secret，配置路径却会崩溃。

**修复（✅ 已应用）**: JWT 路径改用 `BearerAuth::try_new(secret)`，错误映射为 `ConfigError::ValidationError` 返回而非 panic，由调用方决定是否降级/告警。

---

#### 🟡 Medium（1，记录）

- **MED-001** `src/mcp/mrtr.rs:230-262,381-396` — MCP session 表无上限增长，`cleanup_expired` 无调用点；建议接入清理调度 + 容量上限。

#### 🔵 Low / Architecture（2，记录）

- **LOW-001** `build.rs:7,13-17` — proto 生成写入 `src/grpc/pb`（源树内已检入），应改用 `OUT_DIR`，避免 `cargo package` 破坏与生成物漂移。
- **SIMPL-001** `grpc_impl.rs:352` 与 `api_error.rs:411-420` — `InvalidInput` 在 HTTP=400 / gRPC=422 不一致，建议跨协议统一。

---

### 🧬 Decay Risks（Engine B）

| Risk | 发现 |
|---|---|
| R4 偶然复杂度 | **S**: 宏对同一形参语义生成数种 extractor 形态（Path/Query/Header/Form/Body/Streaming），正确性负担集中在生成器 → **C**: 坏形态难以穷尽测试（HIGH-001 即未覆盖）→ **R**: 收敛为"结构体提取 + 有限形态白名单"。 |
| R5 依赖方向 | **S**: `WebSocketRoute` 的 create_fn 从未被消费（HIGH-002）→ **C**: 用户 handler 悬空 → **R**: 注册即消费的单向接线，靠编译/测试兜底。 |
| R3 知识重复 | **S**: HTTP/gRPC/WebSocket 各自独立做认证/错误映射（MED-007 类），同源逻辑三处表达 → **C**: 行为漂移 → **R**: 收敛到 security 模块统一出口。 |

**Health Score**: 74/100。分层清晰，宏 + 运行时 + 安全套件结构良好；主要扣分来自"注册了但未接线"（WebSocket）、宏代码生成正确性覆盖不足、认证缓存值设计未与过期语义对齐。

---

### ✂️ Simplification Opportunities（Engine C）

- `mrtr.rs`: `delete:` 无调用点的 `cleanup_expired` 要么接上调度要么删除。net: -0（待接线）
- 宏 `param_unwraps`/`param_names` 两段未使用向量可删（`_param_unwraps` 已标注"kept for future"）。net: -20 lines possible

---

### Verdict

- [x] ✅ **Approved（有条件）** — 无 Critical；HIGH-004 持久化损坏与 HIGH-005 启动 panic 已修复；HIGH-001/002/003 与 HIGH-004 过期强制为**已知限制**，已在本报告记录并给出各自 Remediation，不静默、不假装已修
- **发布前提（用户决策点）**: 若严格按"无未处置 High"放行，需在 +0.1 发布前完成 HIGH-001（宏重构）、HIGH-002（WS 接线）、HIGH-003（key 播种）、HIGH-004 过期强制——这是一轮专项重构，预计工作量明显；否则本报告作为已知限制放行，并纳入下一版本

---

*修复 commit: 待提交后回填。涉及 http_impl.rs（JWT try_new）、api_key_manager.rs（剩余时长序列化）。*

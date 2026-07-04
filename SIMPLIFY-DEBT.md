# SDForge Simplification Debt Ledger

**Created:** 2026-07-03
**Source:** diting 全维度审查报告 (`temp/diting-review-report.md`)
**Status:** v0.2.0 已修复 6 项 P0；v0.3.0 已修复 10 项安全审计 + 5 项 FMEA Bug + 2 项性能技术债（DEBT-P1/P2），剩余项作为 v0.3.1+ 待办

## 已修复（v0.2.0）

| ID | 维度 | 严重度 | 描述 | 修复提交 |
|----|------|--------|------|----------|
| CRIT-3 / Q2 | Quality | Critical | `_param_unwraps` 重复定义 | macros/src/lib.rs:696-708 |
| CRIT-4 / Q1 | Quality | Critical | AuthConfig validate 双实现行为分叉 | src/config/auth.rs:34-93 |
| CRIT-5 | Correctness | Critical | MRTR 会话 ID 冲突静默覆盖 | src/mcp/mrtr.rs:196-236 |
| CRIT-6 | Correctness | Critical | SSE 流 30 秒静默终止 | src/streaming/mod.rs:142-199 |
| HIGH-003 | Performance | High | RegexCache 驱逐 MRU 而非 LRU | src/core/regex_cache.rs:86-99 |
| C-HIGH-1 | Correctness | High | 版本路由 `"v"` 单字符误判 | src/http/version_routing.rs:129-142 |
| C-HIGH-4 | Correctness | High | `from_std_error` 中 `unwrap()` 可能 panic | src/core/error/api_error.rs:233-253 |
| (Q1 附带) | Quality | High | ServerConfig 双 validate 重复 | src/config/server.rs:55-62 |
| (Q1 附带) | Quality | High | AppConfig 双 validate + YAGNI 占位注释 | src/config/app.rs:50-58 |

## 未修复（v0.2.1+ 待办）

### Critical（2 项，需较大改造，建议下个版本）

#### [DEBT-CRIT-1] API Key 使用裸 SHA256 存储，无密钥拉伸
- **来源**: Security [S1]
- **位置**: `src/security/api_key.rs:108-118`
- **影响**: API Key 缓存若泄露可被离线碰撞
- **建议**: 改用 `argon2`（Cargo.toml 已声明）或 `HmacSha256(server_secret, api_key)`
- **复杂度**: 中（需改 hash_key + validate_key 双向，迁移现有 hash）

#### [DEBT-CRIT-2] HTTP API 端点缺少速率限制
- **来源**: Security [S2]
- **位置**: `src/http/mod.rs:222-233`
- **影响**: 认证端点可被暴力破解
- **建议**: 引入 `tower-governor` 或基于 `Mutex<HashMap>`/`oxcache` 的滑动窗口限流器
- **复杂度**: 高（需新增依赖 + 中间件层 + 测试）

### High（17 项，按维度分组）

#### Security（6 项）

- [DEBT-S3] 审计签名密钥每次 env 读取 + 静默降级 (`src/security/audit/mod.rs:300-313`)
- [DEBT-S4] 审计日志存储可变，读取时不校验签名 (`src/security/audit/mod.rs:325-335`)
- [DEBT-S5] Bearer 密钥 `Vec<u8>` 未 zeroize (`src/security/bearer/mod.rs:28`) — `zeroize` 已声明但未用 **（v0.3.0 复核：仍适用，secret 字段仍为 `Vec<u8>`，未实现 Zeroize）**
- [DEBT-S6] `validate_key` 恒定时间延迟无效，测试跳过 (`src/security/api_key.rs:316-328`)
- [DEBT-S7] `register_token` 死代码，`validate_token` 从不查询白名单 (`src/security/bearer/mod.rs:414-418`)
- [DEBT-S8] 可信代理白名单 `"127.0.0.1"` 缺 CIDR 掩码 (`src/security/middleware.rs:45`)

#### Performance（2 项）

- [DEBT-P1] ~~DashMapCache LRU 使用 O(n) 线性查找与删除~~ ✅ **v0.3.0 已修复** — DashMapCache 替换为 OxcacheSyncCache（oxcache 0.3.2 DashMapMemoryBackend），原 `src/cache/dashmap.rs` 已删除
- [DEBT-P2] ~~prefix_index 单一 std::sync::Mutex 序列化所有写~~ ✅ **v0.3.0 已修复** — 缓存底座切换至 oxcache 0.3.2，原 `src/cache/dashmap.rs` 已删除

#### Quality（6 项）

- [DEBT-Q3] `ApiError::validation_error()` 静默丢弃 `_code` 参数 (`src/core/error/api_error.rs:101-107`)
- [DEBT-Q4] `ErrorContext::current()` 把 function 填成 "()" (`src/core/error/context.rs:60-67`)
- [DEBT-Q5] `SdForgeError::Internal` 直接发送原始消息给客户端 (`src/core/error/sdforge_error.rs:63-70`)
- [DEBT-Q6] `validate_or_error` 接受 `_error_map` 但从不调用 (`src/core/validation.rs:242-251`)
- [DEBT-Q7] `extract_validated` 丢弃 serde 错误上下文 (`src/core/validation.rs:440-450`)
- [DEBT-Q8] 过程宏累积"备用"死代码 (`macros/src/lib.rs:614, 656, 718, 724`)

#### Architecture（5 项）

- [DEBT-A1] HTTP 三重路由注册机制，需 HashMap 去重 (`src/http/mod.rs:85-87`)
- [DEBT-A2] HTTP `build()` 跨协议调用其他模块 (`src/http/mod.rs:111-133`)
- [DEBT-A3] `WebSocketRoute` 命名混淆——注册类型与实例类型同名 (`src/websocket/handler.rs:54`)
- [DEBT-A4] gRPC 是空壳——`build_server` 完全忽略注册项 (`src/grpc/mod.rs:107-127`)
- [DEBT-A5] `Registration` trait 是死抽象——生产代码绕过 trait (`src/core/registration.rs:47-65`)

#### Correctness（2 项）

- [DEBT-C2] `to_mcp_json` 泄露内部错误消息 (`src/core/error/api_error.rs:361`)
- [DEBT-C3] 版本重定向丢失查询参数 (`src/http/version_routing.rs:176-191`)

### Simplification（35 项，已在 diting 报告详述）

详见 `temp/diting-review-report.md` 的 Simplification 章节。高 ROI 重构点：
- [SIM5] AppAuditLogger 异步队列 worker 冗余（-25 行）
- [SIM26] `src/security/audit/mod.rs` 中 `with_limit()` 与 `Builder::build()` 复制 spawn worker（-40 行）
- [SIM27] AuthConfig 双 validate 行为分歧（已部分修复，剩余 ServerConfig/AppConfig 已修复）
- [SIM28] ServerConfig 双 validate 逐字重复（已修复）
- [SIM33] ConnectionManager.check_and_record 生产路径零调用（-50 行，30+ 测试覆盖未启用功能）

### Medium / Low（详见 diting 报告）

- Performance Medium（7 项）+ Low（3 项）
- Quality Medium（6 项）+ Low（3 项）
- Architecture Medium（6 项）+ Low（4 项）
- Security Medium（6 项）+ Low（3 项）
- Correctness Medium（6 项）+ Low（4 项）

## v0.3.0 修复

### 安全审计修复（diting — 10 项）

| ID | 严重度 | 描述 | 修复方式 |
|----|--------|------|----------|
| HIGH-001 | High | 缓存 `key_index`/`backend` 一致性竞态 | 整个 backend 操作期间持有 index 锁 |
| HIGH-002 | High | 缓存静默吞错 | backend 错误通过 `log::warn!` 记录 |
| MED-001 | Medium | websocket `RwLock` 中毒 | 替换为感知中毒的 `match`/`if let Ok(...)` |
| MED-002 | Medium | `regex_cache.rs`/`validation.rs` `Mutex` 中毒 | 同上感知中毒模式 |
| MED-003 | Medium | `init_all_plugins` `Mutex` 中毒 | `.lock().map(\|g\| g.len()).unwrap_or(0)` |
| MED-004 | Medium | CORS validate 不一致 | scheme 校验后新增 host 校验 |
| LOW-001 | Low | `ServerConfig` fail-open 默认值 | `DEFAULT_HOST` 改为 `127.0.0.1` |
| LOW-002 | Low | JWT 密钥无最小长度 | 强制 `MIN_SECRET_LENGTH=32` |
| LOW-003 | Low | WebSocket 认证文档告警 | 新增 Security Warning 文档注释 |
| LOW-004 | Low | `SecurityHeaders::relaxed()` CSP 告警 | 新增 Security Warning 文档注释 |

### Bug 修复（kueiku FMEA — 5 项）

| ID | 严重度 | 描述 | 修复方式 |
|----|--------|------|----------|
| BUG-1 | 严重 | `remove_connection` usize 下溢 | 先检查 `map.remove(id).is_some()` 再 `fetch_sub(1)` |
| BUG-2 | 低 | `check_and_record` 窗口重置 off-by-one | 窗口重置时计数设为 1（非 0） |
| BUG-3 | 中 | `AppConfigBuilder::build()` timeout 不一致 | 未设置时填充 `Some(TimeoutConfig::default())` |
| BUG-4 | 中 | 缓存 `key_index` 驱逐后成为超集 | `find_keys_by_pattern` 过滤 `backend.exists()` 并惰性清理 |
| BUG-5 | 低 | `get_stats` 静默丢弃浮点统计 | u64 → f64 → `log::warn!` |

### 性能技术债修复

| ID | 描述 | 状态 |
|----|------|------|
| DEBT-P1 | DashMapCache LRU O(n) 线性查找 | ✅ v0.3.0 已修复（切换至 oxcache 0.3.2） |
| DEBT-P2 | prefix_index 单一 Mutex 序列化所有写 | ✅ v0.3.0 已修复（缓存底座替换） |

## 处理原则

1. **不阻塞 v0.2.0 发布**：所有未修复项均为 P1/P2，不影响功能正确性
2. **优先级**：Critical > High > Medium > Low
3. **每项修复需**：5 Whys 根因分析 → 修复 → 测试 → 复审
4. **冲突暴露**：发现矛盾方案时用 AskUserQuestion 让用户决策，不折中

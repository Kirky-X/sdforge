# FEATURE_AUDIT_REPORT §6（sdforge）复核记录（ws-R14）

> 复核基准：`/FEATURE_AUDIT_REPORT.md` §6.2（隔离性问题 8 项）+ §6.3 命名 + §6.4 粒度。
> 复核日期：2026-09-30。裁决口径：成立则修；过时则附证据记录。
> 本轮同步新增的 feature（`schemars` / `ratelimit-dist` / `cache-l2` / `db-integration` / `sdk`）的 `full` 归属口径一并在 §1 登记。

## §6.2 逐条裁决

### 1. [高] `full` 覆盖失真 — 部分过时，残留已修

- 审查所指 9 项缺失中 validate/paginate/etag/hooks/lifecycle/otel 已在后续提交入 `full`（`Cargo.toml` `full` 现为 25 项，含 idempotency）。
- 残留修复：README 特性表宣称"24 项"与 Cargo 实际 25 项漂移（`idempotency` 未入文档）→ 已同步为 25 项。
- 口径登记（非缺失）：`simd-json` / `kit` / `limiteron-integration` 与本轮新增的 `schemars`（opt-in 精确度）、`ratelimit-dist` / `cache-l2` / `db-integration`（可选跨仓重依赖）、`sdk`（生成期工具面）不并入 `full`——`full` 语义为"运行时特性全集"，重依赖/精确度选项按需启用，README 特性表已逐行标注。

### 2. [高] build.rs 无条件编译 proto — 过时

- `build.rs` 已以 `CARGO_FEATURE_GRPC` 环境变量包裹 `compile_protos`（带双端门控一致性注释），`default = []` 纯核心构建不再要求 protoc。证据：`build.rs:8-11`。

### 3. [高] 两处死回退门控 — 过时

- `bearer_impl.rs` 的 `cfg(not(feature = "security"))` `constant_time_eq` 回退：已不存在（全仓 grep `not(feature = "security")` 于 `src/security/` 零命中）。
- `mcp/mod.rs` 的 `cfg(not(feature = "mcp"))` stub：已不存在（`src/mcp/` 内零命中；`src/docs/` 中的 `cfg(not(feature = "mcp"))` 是 docs 模块对可选 mcp 面的合法分支，非死回退）。

### 4. [中] cfg 门控公开结构体字段 — 成立，已修（类型切换）

- `GrpcServerConfig` 三字段（`auth` / `auth_verifier` / `rate_limiter`）：改为**字段恒存在 + 类型按 feature 切换**——开启态保持原 `Option<T>`，关闭态为空壳 `Option<()>`（恒 `None`）。结构体字面量 `auth: None` / `rate_limiter: None` 在任意 feature 组合下形态稳定，构造点的 `#[cfg]` 字段属性全部移除（`grpc_impl.rs` Default、`tests/integration/grpc_tests.rs` ×3、`src/grpc/tests/grpc_service_tests.rs`、`examples/src/grpc/server.rs`）。读取点全部位于对应 feature 门控块内（`grpc_impl.rs:1127/1139/1144/1161`），双态编译验证：`--features "http,grpc"` 与 `--features "http,grpc,security,ratelimit,idempotency,grpc-tls"` 均 0 error。
- `PluginCounts`：字段恒存在（`usize`，非 feature 态计数恒 `0`，集合侧 `#[cfg(not)] let x = 0` 孪生绑定）；结构体字面量/打印跨组合形态稳定。三态编译验证（default / http / http+mcp+grpc+websocket+cli）均 0 error。
- 同模式未在本轮处理（超审计范围，登记）：`GrpcServerConfig` 的 `idempotency_store` / `idempotency_ttl_secs` / `idempotency_inflight_ttl_secs` / `tls` 与 `SdForgeGrpcService` 内部字段沿用旧模式，后续卫生轮可按同法收敛。

### 5. [中] 宏发射 cfg 依赖下游镜像 — 部分缓解，已补文档

- examples 已从裸 `#![allow(unexpected_cfgs)]` 升级为 `[lints.rust]` `check-cfg` 白名单（`examples/Cargo.toml:17-23`）。
- README「特性依赖关系」节补官方片段：下游按实际镜像 feature 裁剪 `check-cfg` 白名单。宏改能力探测不做（inventory 发射的 cfg 门控是既定架构，探测方案无收益）。

### 6. [中] 模块内约 70 处冗余再门控 — 成立，已修（机械清理）

- 同名单一 feature 的行内 `#[cfg(feature = "X")]` 属性行删除：`src/mcp/mod.rs` −23、`src/grpc/grpc_impl.rs` −22、`src/grpc/mod.rs` −10、`src/websocket/connection.rs` −11（共 66 处；父模块已在 `lib.rs` 声明处门控，内部重复门稀释信噪比）。
- 保留：`grpc_impl.rs:1247` `#[cfg(all(test, feature = "grpc"))]`（tests 模块的惯例形态）与各文件内**跨 feature** 的门（如 grpc 文件内的 `cfg(security)` / `cfg(ratelimit)` / `cfg(idempotency)`——它们不是父模块门的重复）。
- 验证：default / mcp / grpc / websocket 单 feature 与 `--all-features` check 均 0 error。

### 7. [中] security 冗余使能 `oxcache/minimal` — 过时

- 现 `security` feature 定义（`Cargo.toml:361-376`）已无 `oxcache/minimal`，仅保留 `cache`。`dep:hex` 已并入 `security`（审查 §6.2 [低] 项同步过时：独立 `hex` feature 已删除）。

### 8. [低] 杂项 — 部分过时，残留已修

- `html_root_url` 写死 rc.2（审查记 rc.3，实际更旧）→ 修正为 `0.5.0-rc.6`（`macros/src/lib.rs:7`）。
- examples 未用 workspace 依赖：仍未收敛（examples 成员依赖管理独立演进，非缺陷，登记）。

## §6.3 命名

- `docs` → `docgen`、`kit` → `trait-kit`、`graceful` → `graceful-shutdown`：均涉及下游破坏性迁移，收益为纯命名美学，裁决**保留现名**（`docs` 保留子命令语义已文档化；`kit` 在 README/CHANGELOG 双向标注对应 trait-kit）。
- `hex`：随 §7 过时（已无独立 feature）。
- 其余命名审查结论（17 个轻能力名合格、ratelimit 分层语义真实）无动作。

## §6.4 粒度

- `full` 语义失真、死回退、模块内冗余门：随 §6.2 §1/§3/§6 处理。
- `lib.rs:488-584` 七段聚合上移为条目级 `fn count_x()`：**成立，未做**——纯重构无行为增益，本轮聚焦审计问题项；登记为后续卫生轮。
- `docs_impl.rs` 三处占位串块条目级助手：同上，登记。

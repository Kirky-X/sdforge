# 🧪 Sdforge 测试场景

本文档固化 SDForge 的测试金字塔基线、测试目标落点、E2E 场景定义、组合矩阵与静态门槛。验证口径全部为 `cargo test`（全量口径 `--workspace --all-features`）。

## 📊 测试金字塔基线

| 层级 | 承载 | 数量基线 |
|------|------|----------|
| L1 lib 单元测试 | `src/**` 内 `#[cfg(test)]`：sdforge 2205 / sdforge-macros 85 / sdforge-examples 41 | 2331 passed |
| L2 集成测试 | 主 crate `tests/` 49 目标（integration/ 40 + unit/ 5 + 顶层 2：integration_http_mcp、sdk_snapshots + macros/macro_tests 1 + e2e 1：e2e_advanced，其 157 计入 L3）+ sdforge-macros crate `tests/` 2 目标（macro_compile/trybuild）+ sdforge-examples crate `tests/` 3 目标（comprehensive_features、dbnexus_gateway_e2e、oxcache_admin_e2e） | 688 passed |
| L3 E2E 场景 | `tests/e2e/`（e2e_advanced 单目标，11 域） | 157 passed |
| L4 Doc-tests | sdforge 23 + sdforge-examples 5 | 28 passed + 45 ignored |

全量结果（`--all-features --workspace`，2026-11-16 实测）：**3204 passed / 0 failed / 73 ignored**（ignored = grpc_tests 28 环境门控 + Doc-tests 45 文档示例门控）。

## 🎯 测试目标落点

- 主 crate：49 个测试目标全部 `[[test]]` 显式注册（tests/e2e、tests/integration、tests/unit、tests/macros 子目录均脱离 Cargo 自动发现范围，不注册即静默消失；顶层 integration_http_mcp 亦注册；`test_target_inventory_tests` 持续断言本行与 integration/ 文件数的漂移）
- sdforge-macros / sdforge-examples crate：`tests/` 顶层自动发现（macro_compile_tests、trybuild_tests、comprehensive_features、dbnexus_gateway_e2e、oxcache_admin_e2e）
- e2e_advanced：由 `tests/e2e/` 目录承载（e2e_* 不裸放顶层）；子目录脱离自动发现范围，经 `[[test]]` 显式注册，157 测试完整保留
- 特性门控：e2e_advanced 内 11 mod 独立 `#[cfg(feature)]`，任意 feature 子集可编译（无需 required-features）；全量口径 `--all-features` 无静默跳过风险

## 🧭 E2E 场景定义（11 域，157 测试，0 ignored）

tests/e2e/e2e_advanced.rs（场景要点取自各 mod 声明的公共 API）：

| 域 | 数量 | 场景要点 |
|----|------|----------|
| cache_advanced | 13 | DashMapCache / SyncCache 高级缓存语义 |
| config_advanced | 19 | 配置模块高级语义 |
| error_advanced | 23 | core::ServiceError 分类/上下文/错误链 |
| ratelimit_error_advanced | 8 | ApiError 限流错误族映射（exceeded/banned/circuit/quota/溢出饱和） |
| i18n_advanced | 26 | HttpI18nFormatter / parse_accept_language / I18nError 多语言回退 |
| security_advanced | 14 | JwtError / AuthError / AuthConfigError 错误族 |
| ratelimit_advanced | 9 | RateLimiter / RateLimitError 限流语义 |
| regex_cache_advanced | 15 | RegexCache / common / get_regex 缓存语义 |
| plugin_init_advanced | 4 | init_all_plugins 全插件初始化 |
| cross_module_advanced | 24 | ApiError + LocalizedError + SdForgeError + TranslationStore 跨模块联动 |
| validation_constants_advanced | 2 | core::validation 校验常量边界 |

## 📐 组合矩阵

| 组合 | 覆盖 | 结果 |
|------|------|------|
| `--all-features --workspace`（全量口径） | 全部 59 个有产出目标 | 3204 passed / 0 failed / 73 ignored |
| 七特性子集 `http,security,cache,inklog,i18n,ratelimit,openapi` | e2e_advanced 11 域 | 157 passed / 0 failed |
| trybuild | 宏编译失败诊断用例 | 2 passed |

59 个有产出目标 = 主 crate 49 + sdforge-macros tests 2 + sdforge-examples tests 3 + unittests 3（sdforge/macros/examples）+ Doc-tests 2（sdforge/sdforge-examples；sdforge-macros Doc-tests 空产出不计入）。

## 🚦 静态门槛

| 门槛 | 命令口径 | 结果 |
|------|----------|------|
| fmt | `cargo fmt --all -- --check` | 净 |
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 零告警 |
| doc | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` | 零告警 |
| deny | `cargo deny check` | 4 项 ok（licenses clarify ×4） |
| audit | `cargo audit` | rc=0（bincode 已迁移 postcard 出树，RUSTSEC-2025-0141 不再出现，豁免已清除） |
| MSRV | rust-version（workspace 统一 1.97.1） | 一致 |

## ℹ️ 统计口径与 Ignored 项

- **统计口径**：全量 3204 按各测试目标的 `cargo test` 输出求和，`0 passed; 0 failed` 的空目标不计入有产出目标数。
- **grpc_tests 28 ignored**：上游 `#[ignore]` 标注真实网络绑定 `127.0.0.1:0` 在 CI / 沙箱环境会挂起，属环境门控而非缺陷，予以保留。
- **Doc-tests 45 ignored**：文档示例依赖运行上下文，按门控跳过（sdforge 43 + sdforge-macros 2）。
- **cargo-deny 许可证核对**：`deny.toml` 中 sdforge / sdforge-macros / limiteron / oxcache 四个成员以 license-file 形式声明许可证（cargo-deny 不识别该形态），已通过 clarify 绑定 MIT hash 0xfb13e7ad；license-file 路径相对被 clarify crate 的 manifest 目录解析（sdforge-macros 位于 `macros/` 子目录，使用 `../LICENSE` 回退一级）。advisories / bans / sources 检查均通过。

## 🧪 测试策略矩阵与运行命令

### 测试策略矩阵

<details>
<summary>📋 表格</summary>

| 层级 | 位置 | 说明 |
|------|------|------|
| 单元测试 | `src/` 内嵌 `#[cfg(test)]`、`tests/unit/` | 模块级测试，含 proptest 属性测试（`src/tests/property_tests.rs`） |
| 集成测试 | `tests/integration/` | 覆盖 http / mcp / grpc / websocket / security / cache / streaming / openapi / cli / docs / health_probes / graceful_shutdown / http_tls / validate / paginate / etag / lifecycle / otel_export / status_code / rbac 等协议与特性组合 |
| 宏测试 | `tests/macros/`、`macros/tests/` | trybuild 编译失败用例与宏展开验证 |
| E2E | `tests/e2e/` | `e2e_advanced` 多域 E2E 场景（11 域规模基线见上文「E2E 场景定义」一节） |
| 示例综合测试 | `examples/tests/` | 全 feature re-export 与跨协议 dispatch（76 个测试）及网关 E2E |
| 基准测试 | `benches/`、`src/benches/` | criterion：`runtime_bench` / `config_and_cache_bench` / `sdforge_bench` |
| Doc-tests | `src/` 文档注释 | rustdoc 内嵌示例 |

</details>

### 运行命令

<details>
<summary>📄 代码 · bash</summary>

```bash
# 与 CI 矩阵一致（http / mcp / http,mcp / http,security / http,cache /
# http,websocket / http,grpc / http,streaming / full 共 9 种组合）。
# 组合矩阵只测 sdforge 本体（-p sdforge）：--workspace 会因 sdforge-examples
# 的默认特性触发 feature 统一，把 sdforge 的几乎全部特性带进每个组合。
cargo test -p sdforge --features "http,mcp"

# 工作区全量（sdforge + sdforge-macros + sdforge-examples，含全部特性）
cargo test --workspace --all-features

# lib 测试（CI 覆盖率口径）
cargo test --features full --lib

# 覆盖率（CI 门禁 ≥90% 行覆盖：--features full --lib；lefthook pre-push 仅沿用同一阈值，未限定 --features full --lib）
cargo llvm-cov --features full --lib --lcov --fail-under-lines 90

# 格式化与零告警 Lint
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

</details>

build.rs 生成的 protobuf 代码写入 `OUT_DIR`，不再落源树；`llvm-cov.toml` 中的 `src/grpc/pb/*` 排除项为旧布局残留，当前不匹配任何文件。

### 测试规模

约 **3,262** 个测试函数（`src/` 2,251 + `tests/` 795 + `macros/` 92 + `examples/` 124，`grep -rEc '#\[(tokio::)?test\b' --include='*.rs'` 统计，2026-11-16 实测）。

# 🧪 Sdforge 测试场景

本文档固化 SDForge 的测试金字塔基线、测试目标落点、E2E 场景定义、组合矩阵与静态门槛。验证口径全部为 `cargo test`（全量口径 `--workspace --all-features`）。

## 📊 测试金字塔基线

| 层级 | 承载 | 数量基线 |
|------|------|----------|
| L1 lib 单元测试 | `src/**` 内 `#[cfg(test)]`：sdforge 1877 / sdforge-macros 49 / sdforge-examples 44 | 1970 passed |
| L2 集成测试 | 主 crate `tests/` 26 目标（integration/ 19 + unit/ 5 + 顶层 integration_http_mcp 1 + macros/macro_tests 1）+ sdforge-macros crate `tests/` 2 目标（macro_compile/trybuild）+ sdforge-examples crate `tests/` comprehensive_features | 532 passed |
| L3 E2E 场景 | `tests/e2e/`（e2e_advanced 单目标，12 域） | 178 passed |
| L4 Doc-tests | sdforge 22 + sdforge-examples 5 | 27 passed + 32 ignored |

全量结果（`--all-features --workspace`）：**2707 passed / 0 failed / 60 ignored**（ignored = grpc_tests 28 环境门控 + Doc-tests 32 文档示例门控）。

## 🎯 测试目标落点

- 主 crate：27 个测试目标全部 `[[test]]` 显式注册（tests/e2e、tests/integration、tests/unit、tests/macros 子目录均脱离 Cargo 自动发现范围，不注册即静默消失；顶层 integration_http_mcp 亦注册）
- sdforge-macros / sdforge-examples crate：`tests/` 顶层自动发现（macro_compile_tests、trybuild_tests、comprehensive_features）
- e2e_advanced：由 `tests/e2e/` 目录承载（e2e_* 不裸放顶层）；子目录脱离自动发现范围，经 `[[test]]` 显式注册，178 测试完整保留
- 特性门控：e2e_advanced 内 12 mod 独立 `#[cfg(feature)]`，任意 feature 子集可编译（无需 required-features）；全量口径 `--all-features` 无静默跳过风险

## 🧭 E2E 场景定义（12 域，178 测试，0 ignored）

tests/e2e/e2e_advanced.rs（场景要点取自各 mod 声明的公共 API）：

| 域 | 数量 | 场景要点 |
|----|------|----------|
| cache_advanced | 13 | DashMapCache / SyncCache 高级缓存语义 |
| config_advanced | 19 | 配置模块高级语义 |
| error_advanced | 23 | core::ServiceError 分类/上下文/错误链 |
| ratelimit_error_advanced | 8 | ApiError 限流错误族映射（exceeded/banned/circuit/quota/溢出饱和） |
| logging_advanced | 21 | logging 模块高级语义 |
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
| `--all-features --workspace`（全量口径） | 全部 35 个有产出目标 | 2707 passed / 0 failed / 60 ignored |
| 七特性子集 `http,security,cache,logging,i18n,ratelimit,openapi` | e2e_advanced 12 域 | 178 passed / 0 failed |
| trybuild | 宏编译失败诊断用例 | 2 passed |

35 个有产出目标 = 主 crate 27 + sdforge-macros tests 2 + sdforge-examples tests 1 + unittests 3（sdforge/macros/examples）+ Doc-tests 2。

## 🚦 静态门槛

| 门槛 | 命令口径 | 结果 |
|------|----------|------|
| fmt | `cargo fmt --all -- --check` | 净 |
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 零告警 |
| doc | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` | 零告警 |
| deny | `cargo deny check` | 4 项 ok（licenses clarify ×4） |
| audit | `cargo audit` | rc=0（bincode RUSTSEC-2025-0141 已豁免，见 deny.toml） |
| MSRV | rust-version（workspace 统一 1.97.1） | 一致 |

## ℹ️ 统计口径与 Ignored 项

- **统计口径**：全量 2707 按各测试目标的 `cargo test` 输出求和，`0 passed; 0 failed` 的空目标不计入有产出目标数。
- **grpc_tests 28 ignored**：上游 `#[ignore]` 标注真实网络绑定 `127.0.0.1:0` 在 CI / 沙箱环境会挂起，属环境门控而非缺陷，予以保留。
- **Doc-tests 32 ignored**：文档示例依赖运行上下文，按门控跳过。
- **cargo-deny 许可证核对**：`deny.toml` 中 sdforge / sdforge-macros / limiteron / oxcache 四个成员以 license-file 形式声明许可证（cargo-deny 不识别该形态），已通过 clarify 绑定 MIT hash 0xfb13e7ad；license-file 路径相对被 clarify crate 的 manifest 目录解析（sdforge-macros 位于 `macros/` 子目录，使用 `../LICENSE` 回退一级）。advisories / bans / sources 检查均通过。

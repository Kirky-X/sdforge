# sdforge 测试场景固化（TEST_SCENARIOS）

> 阶段 2 验收产物。记录测试金字塔基线、测试目标落点、E2E 场景定义、本轮核正发现、
> 组合矩阵与静态门槛。验证口径全部为 `cargo test`（全量口径
> `--workspace --all-features`）。

## §1 测试金字塔基线

| 层级 | 承载 | 数量基线 |
| --- | --- | --- |
| L1 lib 单元测试 | `src/**` 内 `#[cfg(test)]`：sdforge 1877 / sdforge-macros 49 / sdforge-examples 44 | 1970 passed |
| L2 集成测试 | 主 crate `tests/` 26 目标（integration/ 19 + unit/ 5 + 顶层 integration_http_mcp 1 + macros/macro_tests 1）+ sdforge-macros crate `tests/` 2 目标（macro_compile/trybuild）+ sdforge-examples crate `tests/` comprehensive_features | 532 passed |
| L3 E2E 场景 | `tests/e2e/`（e2e_advanced 单目标，12 域） | 178 passed |
| L4 Doc-tests | sdforge 22 + sdforge-examples 5 | 27 passed + 32 ignored |

全量结果（`--all-features --workspace`）：**2707 passed / 0 failed / 60 ignored**
（ignored = grpc_tests 28 环境门控 + Doc-tests 32 文档示例门控）。

## §2 测试目标落点

- 主 crate：27 个测试目标全部 `[[test]]` 显式注册（tests/e2e、tests/integration、
  tests/unit、tests/macros 子目录均脱离 Cargo 自动发现范围，不注册即静默消失；
  顶层 integration_http_mcp 亦注册）
- sdforge-macros / sdforge-examples crate：`tests/` 顶层自动发现
  （macro_compile_tests、trybuild_tests、comprehensive_features）
- e2e_advanced：本轮从 tests/ 顶层迁入 tests/e2e/ 目录承载（e2e_* 不得裸放
  顶层）；迁入子目录后脱离自动发现范围，`[[test]]` 注册后 178 测试零损失
- 特性门控：e2e_advanced 内 12 mod 独立 `#[cfg(feature)]`（任意 feature 子集
  可编译，无需 required-features）；全量口径 `--all-features` 无静默跳过风险

## §3 E2E 场景定义（12 域，178 测试，0 ignored）

tests/e2e/e2e_advanced.rs（场景要点取自各 mod 声明的公共 API）：

| 域 | 数量 | 场景要点 |
| --- | --- | --- |
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

## §4 本轮核正与发现（阶段 2）

1. **e2e_advanced 目录承载迁移**：tests/e2e_advanced.rs 裸放顶层（违反 e2e_*
   目录承载约定）→ mkdir tests/e2e + git mv 迁入 + `[[test]]` 显式注册；迁移
   前后 178 passed 一致（零损失）。主 crate 27 目标注册齐全，无死文件。
2. **deny licenses clarify ×4**：sdforge/sdforge-macros/limiteron/oxcache（工作区
   license-file 形态不被 cargo-deny 识别）→ clarify 绑定 MIT hash 0xfb13e7ad；
   path 相对被 clarify crate manifest 目录解析（sdforge-macros 为 macros/ 子
   crate，用 "../LICENSE" 回退一级）。advisories/bans/sources 原本即 ok。
3. **统计口径核正**：awk 汇总以 "0 passed; 0 failed" 过滤空目标，但 "10 passed"
   包含该子串被误过滤（http_version_routing_tests）→ 单独复验 10 passed 补齐，
   总数 2707。
4. **grpc_tests 28 ignored**：上游原有 `#[ignore = "environmental issue: real
   network binding to 127.0.0.1:0, hangs in CI/sandboxed environments"]`——真实
   网络绑定在 CI/沙箱环境挂起，环境门控保留（非本轮引入）。
5. **跨 crate 目标归属核实**：comprehensive_features（sdforge-examples）、
   macro_compile_tests / trybuild_tests（sdforge-macros）为 workspace 成员
   `tests/` 自动发现，非死文件。

## §5 组合矩阵

| 组合 | 覆盖 | 结果 |
| --- | --- | --- |
| --all-features --workspace（全量口径） | 全部 35 个有产出目标 | 2707 passed / 0 failed / 60 ignored |
| 七特性子集 http,security,cache,logging,i18n,ratelimit,openapi | e2e_advanced 12 域 | 178 passed / 0 failed（迁移验证口径） |
| trybuild | 宏编译失败诊断用例 | 2 passed |

35 个有产出目标 = 主 crate 27 + sdforge-macros tests 2 + sdforge-examples
tests 1 + unittests 3（sdforge/macros/examples）+ Doc-tests 2。

## §6 静态门槛

| 门槛 | 命令口径 | 结果 |
| --- | --- | --- |
| fmt | `cargo fmt --all -- --check` | 净 |
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 零告警 |
| doc | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` | 零告警 |
| deny | `cargo deny check` | 4 项 ok（licenses clarify ×4） |
| audit | `cargo audit` | rc=0（bincode RUSTSEC-2025-0141 已豁免，见 deny.toml） |
| MSRV | rust-version（workspace 统一 1.97.1） | 一致 |

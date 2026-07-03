# SDForge vs Server-less 性能对比

## 概述

SDForge 是一个声明式 SDK 框架，核心设计理念之一是 **编译时协议选择——未使用的协议产生零编译代码**。通过 Cargo feature flag 与 `#[cfg(feature = "...")]` 的组合，框架仅在启用对应协议时才将其实现代码与依赖纳入编译图。

本文档对比两种交付模式：

- **SDForge（特性门控）**：按需启用协议，默认仅编译 HTTP（`cargo build`），未启用的 MCP / WebSocket / gRPC / streaming / security / cache 等模块其源码与依赖完全不进入编译产物。
- **Server-less（全量打包）**：对应 SDForge 的 `--features full`，把所有协议实现与依赖一次性编译进产物。这也是多数"开箱即用"框架的默认形态——无法在编译期裁剪未用功能。

目标是量化特性门控带来的 **编译时间** 与 **产物体积** 优势。

## 方法论

- **测量方法**：每次测量前执行 `cargo clean` 清空 `target/`，随后 `cargo build` 计时。编译时间取 cargo 自身报告的 `Finished ... in Xs`；同时记录 wall-clock（含 cargo 启动/链接开销）作为参考。
- **测量场景**：`--features http`（默认，仅 HTTP）vs `--features full`（全量协议）。
- **构建模式**：`dev`（debug）profile 与 `release` profile。release profile 配置为 `lto = true`、`codegen-units = 1`、`opt-level = "z"`（体积优化）。
- **体积指标**：
  - `libsdforge.rlib`——框架库编译产物，是反映框架代码体积最干净的单一指标；
  - `target/debug|release/deps` 与 `target/` 目录总大小——反映全部依赖编译产物的磁盘占用；
  - `sdforge` 二进制——注意：`full` 特性 **不含** `cli`，因此默认二进制是 `panic` stub，需另行测量 `--features <x>,cli` 才能得到真实 CLI 二进制体积。
- **依赖规模**：用 `cargo tree` 统计去重后的唯一 crate 数。
- 每个场景独立测量，未跨场景复用缓存。

## 基准环境

| 项目 | 值 |
|------|-----|
| CPU | AMD Ryzen 9 9950X（WSL2 分配 12 vCPU：6 核 × 2 线程），~4.29 GHz |
| Rust 工具链 | rustc 1.93.1 (01f6ddf75 2026-02-11) / cargo 1.93.1 |
| 操作系统 | Linux 6.6.87.2-microsoft-standard-WSL2 (x86_64) |
| 内存 | 70 GiB 总计 / 60 GiB 可用 |
| 构建配置 | release: `lto=true`, `codegen-units=1`, `opt-level="z"`；dev: `debug=true`, 依赖 `opt-level=2` |

> 注：`full` 特性集合为 `http, mcp, streaming, timestamp, security, hot-reload, cache, websocket, grpc, logging, openapi`，**不包含** `cli` 与 `simd-json`。

## 依赖规模对比

| 指标 | http only | full | 差异 |
|------|-----------|------|------|
| `cargo tree` 总行数 | 862 | 1129 | +267 行 (+31%) |
| 去重后唯一 crate 数 | 396 | 478 | +82 crate (+20.7%) |

`full` 额外引入的典型重型依赖：`tonic`/`prost`/`tonic-prost`（gRPC）、`tokio-tungstenite`（WebSocket）、`rmcp`（MCP）、`argon2`/`sha2`/`hmac`/`secrets`（security）、`utoipa`（openapi）、`oxcache`（cache）等。这些在 `http only` 构建中完全不出现在依赖图里。

## 编译时间对比

编译时间取 cargo 报告值（`Finished ... in Xs`），wall-clock 为含进程启停的端到端耗时。

| 特性组合 | Debug 编译时间 | Release 编译时间 | Debug wall-clock | Release wall-clock |
|----------|---------------|-----------------|------------------|--------------------|
| http only | 28.88s | 13.72s | 30.07s | 17.11s |
| full | 54.52s | 25.48s | 59.98s | 28.81s |
| 倍数（full / http） | 1.89× | 1.86× | 2.00× | 1.69× |
| http 相对 full 节省 | **47.0%** | **46.2%** | 49.9% | 40.6% |

**结论**：无论 debug 还是 release，`http only` 都比 `full` 节省约 **46–47%** 的编译时间。Debug 模式下省下约 25.6s，release 模式下省下约 11.8s。

## 二进制 / 产物体积对比

### 框架库体积（`libsdforge.rlib`，最干净的框架代码指标）

| 特性组合 | Debug rlib | Release rlib |
|----------|-----------|--------------|
| http only | 33.90 MB（35,540,214 B） | 3.57 MB（3,747,912 B） |
| full | 100.16 MB（105,027,940 B） | 9.07 MB（9,511,678 B） |
| 倍数（full / http） | 2.96× | 2.54× |
| http 相对 full 节省 | **66.2%** | **60.6%** |

### 构建产物总占用（`target/` 目录）

| 特性组合 | Debug deps | Debug target | Release deps | Release target |
|----------|-----------|--------------|--------------|----------------|
| http only | 808 MB | 1.1 GB | 575 MB | 608 MB |
| full | 1.5 GB | 2.0 GB | 792 MB | 832 MB |
| 倍数（full / http） | 1.86× | 1.82× | 1.38× | 1.37× |

### 独立 `sdforge` 二进制体积

| 特性组合 | Debug 二进制 | Release 二进制 |
|----------|-------------|----------------|
| http only | 3.73 MB（3,911,120 B） | 349 KB（357,296 B） |
| full | 3.73 MB（3,911,120 B，**完全相同**） | 349 KB（357,296 B，**完全相同**） |

> **重要说明**：`full` 特性不含 `cli`，因此 `src/main.rs` 在两种场景下都退化成同一个 `panic` stub（`#[cfg(not(feature = "cli"))] fn main()`）。这意味着 **默认二进制体积无法反映协议裁剪的差异**，必须以 `rlib` 或真实 CLI 二进制来评估。

### 真实 CLI 二进制体积（追加 `cli` 特性，release）

| 特性组合 | Release 二进制 |
|----------|----------------|
| http,cli | 5.76 MB（6,037,008 B） |
| full,cli | 5.90 MB（6,183,936 B） |
| 倍数（full / http） | 1.024× |
| http 相对 full 节省 | 2.4% |

> 即便启用 `cli`，最终二进制仅差 2.4%：release 的 `lto=true` + `opt-level="z"` + `codegen-units=1` 对二进制做了激进的死代码消除，未被 `main.rs` 实际引用的协议实现会在链接期被剔除。**但这只影响最终二进制体积，编译期的依赖编译成本依旧要付**（见上表编译时间）。

## 分析

### 1. 编译时间差异的原因

`full` 比 `http` 多引入 82 个唯一 crate（+20.7%），其中 gRPC（tonic/prost）、WebSocket（tokio-tungstenite）、MCP（rmcp）、安全（argon2/sha2/hmac）和 OpenAPI（utoipa）都是代码生成量大、过程宏密集的重型依赖。Debug 模式下 full 多耗时 89%（54.52s vs 28.88s），因为 debug 保留全部符号与调试信息、不做优化；release 下差异略收窄到 86%（25.48s vs 13.72s），LTO 与单 codegen-unit 让两者都变慢，但 full 的额外依赖编译时间依然存在。**编译时间的节省（约 46–47%）是特性门控最直接、最稳定的收益**，且不受最终链接器死代码消除影响。

### 2. 产物体积差异的原因

`rlib` 是反映框架代码体积最准确的单一指标：debug 下 full 是 http 的 2.96×（100.2 MB vs 33.9 MB），release 下是 2.54×（9.07 MB vs 3.57 MB）。差异完全来自被裁剪掉的协议实现及其内联进 rlib 的依赖代码。`target/` 目录总占用 debug 下从 1.1 GB 增至 2.0 GB（+82%），release 下从 608 MB 增至 832 MB（+37%）——这对 CI 缓存与磁盘预算有实际影响。

### 3. 零开销抽象的实际效果

数据印证了"未使用协议产生零编译代码"：在 `http only` 构建中，MCP/WebSocket/gRPC/security 等模块的源码既不出现在 `cargo tree`（396 vs 478 crate），也不出现在 `libsdforge.rlib`（33.9 MB vs 100.2 MB）。这不是运行时判断或运行时加载，而是 **编译期完全裁剪**——未启用特性的代码根本不会被 rustc 处理。量化收益：debug 下省 47% 编译时间、66% 库体积；release 下省 46% 编译时间、61% 库体积。

### 4. 二进制体积评估的注意事项

默认 `sdforge` 二进制在 http 与 full 下完全相同（均为 panic stub），**不能**用作特性裁剪的体积证据。真实 CLI 二进制（追加 `cli`）release 下仅差 2.4%，因为链接器死代码消除了未引用代码。这揭示一个重要区别：
- **编译期成本**（编译时间、依赖图、rlib 体积）由 feature flag 决定，特性门控收益显著；
- **最终二进制体积**还受 LTO/死代码消除影响，特性门控的边际收益在二进制层面被部分稀释——但前提是你愿意为 `full` 付完整编译时间。

### 5. 对开发体验与 CI/CD 的影响

- **日常开发**：默认 `http only` 全量编译 28.88s（debug），比 `full` 快近一倍；增量编译受益更明显（改一个文件不会触发 gRPC/WebSocket 等无关依赖重编）。
- **CI 矩阵**：可按目标协议组合构建（如仅 `http`、`http,websocket`、`mcp`），缩短总流水线时间、降低缓存压力（target 目录小 37–82%）。
- **产物体积敏感场景**（嵌入式/边缘/容器镜像）：release rlib 从 9.07 MB 降到 3.57 MB，对镜像分层与冷启动有正向收益。

## 结论

SDForge 的编译时特性门控实现了真正的"未使用协议产生零编译代码"，量化收益如下：

| 维度 | http only 相对 full 的节省 |
|------|---------------------------|
| 编译时间（debug） | 47.0%（28.88s vs 54.52s） |
| 编译时间（release） | 46.2%（13.72s vs 25.48s） |
| 框架库体积 rlib（debug） | 66.2%（33.9 MB vs 100.2 MB） |
| 框架库体积 rlib（release） | 60.6%（3.57 MB vs 9.07 MB） |
| 构建产物总占用（debug target） | 45%（1.1 GB vs 2.0 GB） |
| 唯一依赖 crate 数 | 17.2%（396 vs 478） |

相比 server-less（全量打包）方案，SDForge 让仅需要 HTTP 的用户 **少编译 82 个 crate、少付近一半编译时间、库体积缩减约六成**，且这一收益发生在编译期、零运行时开销。最终二进制体积在 LTO 死代码消除下差异较小（CLI 二进制仅 2.4%），但编译期成本与磁盘占用的节省是确定且显著的。

## 复现方法

> 在 SDForge 仓库根目录执行。每次测量前 `cargo clean` 以确保从零编译。

```bash
# 依赖规模
cargo tree --features http --prefix none | sort -u | wc -l   # 396
cargo tree --features full --prefix none | sort -u | wc -l   # 478

# === Debug 编译时间 + rlib 体积 ===
cargo clean && cargo build --features http 2>&1 | tail -1
ls -l target/debug/libsdforge.rlib

cargo clean && cargo build --features full 2>&1 | tail -1
ls -l target/debug/libsdforge.rlib

# === Release 编译时间 + rlib 体积 ===
cargo clean && cargo build --features http --release 2>&1 | tail -1
ls -l target/release/libsdforge.rlib

cargo clean && cargo build --features full --release 2>&1 | tail -1
ls -l target/release/libsdforge.rlib

# === 真实 CLI 二进制体积（release）===
cargo clean && cargo build --features http,cli --release 2>&1 | tail -1
ls -l target/release/sdforge

cargo clean && cargo build --features full,cli --release 2>&1 | tail -1
ls -l target/release/sdforge

# === 构建产物总占用 ===
du -sh target/debug target/release
du -sh target/debug/deps target/release/deps
```

> 测量日期：2026-07-03。数据来自上述环境下的实测，不同机器/工具链版本结果会有差异，趋势（http 显著快于 full、rlib 显著小于 full）应保持一致。

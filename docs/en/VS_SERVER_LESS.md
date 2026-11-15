# 📊 Sdforge Compile-Time Gating Benchmarks

This report quantifies the difference between SDForge's two delivery modes — compile-time feature gating (protocols enabled on demand) and full bundling (`--features full`) — in terms of **compile time** and **artifact size**. All data is measured; measurement date 2026-07-03; reproduction commands at the end. Runtime hot-path baselines are in the [performance baselines](PERFORMANCE.md).

## 📋 Table of Contents

<details open>
<summary>📑 Table of Contents</summary>

- [Overview](#-overview)
- [Methodology](#-methodology)
- [Benchmark Environment](#-benchmark-environment)
- [Dependency Scale Comparison](#-dependency-scale-comparison)
- [Compile Time Comparison](#-compile-time-comparison)
- [Binary / Artifact Size Comparison](#-binary--artifact-size-comparison)
- [Analysis](#-analysis)
- [Conclusion](#-conclusion)
- [Reproduction](#-reproduction)

</details>

## 🧭 Overview

SDForge is a declarative SDK framework, and one of its core design principles is **compile-time protocol selection: unused protocols produce zero compiled code**. Through the combination of Cargo feature flags and `#[cfg(feature = "...")]`, the framework only pulls a protocol's implementation code and dependencies into the compile graph when that protocol is enabled.

This document compares the two delivery modes:

- **SDForge (feature gating)**: protocols enabled on demand; by default only HTTP is compiled (`cargo build`), and the source and dependencies of the disabled MCP / WebSocket / gRPC / streaming / security / cache modules never enter the build artifact.
- **Server-less (full bundling)**: corresponds to SDForge's `--features full`, compiling all protocol implementations and dependencies into the artifact at once. This is also the default shape of most "batteries included" frameworks: unused functionality cannot be trimmed at compile time.

The goal is to quantify the **compile time** and **artifact size** advantages of feature gating.

## 🔬 Methodology

- **Measurement method**: `cargo clean` empties `target/` before each measurement, then `cargo build` is timed. Compile time uses cargo's own `Finished ... in Xs` report; wall-clock time (including cargo startup/link overhead) is recorded as reference.
- **Measurement scenarios**: `--features http` (default, HTTP only) vs `--features full` (all protocols).
- **Build profiles**: `dev` (debug) and `release`. At measurement time the release profile was `lto = true`, `codegen-units = 1`, `opt-level = "z"` (size-optimized); the repo's release profile has since changed to `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `strip = true` (see Cargo.toml). The size/time data in this document was not re-measured under the new profile.
- **Size metrics**:
  - `libsdforge.rlib`: the framework library build artifact, the cleanest single metric of framework code size;
  - total size of `target/debug|release/deps` and `target/`: reflects the disk footprint of all dependency build artifacts;
  - the `sdforge` binary: at measurement time the `full` feature set did **not** include `cli`, and the default binary was a `panic` stub; you must additionally measure `--features <x>,cli` to get a real CLI binary size. **The repo currently has no root-package binary target** (`src/main.rs` was removed in v0.3.0, 2026-07-04, and Cargo.toml has no `[[bin]]`); the binary data and corresponding reproduction commands below are historical and no longer executable — use `libsdforge.rlib` for framework code size.
- **Dependency scale**: the count of deduplicated unique crates via `cargo tree`.
- Each scenario was measured independently; no cache was reused across scenarios.

## 🖥️ Benchmark Environment

| Item | Value |
|------|-----|
| CPU | AMD Ryzen 9 9950X (WSL2 allocated 12 vCPU: 6 cores × 2 threads), ~4.29 GHz |
| Rust toolchain | rustc 1.93.1 (01f6ddf75 2026-02-11) / cargo 1.93.1 |
| OS | Linux 6.6.87.2-microsoft-standard-WSL2 (x86_64) |
| Memory | 70 GiB total / 60 GiB available |
| Build configuration (at measurement time) | release: `lto=true`, `codegen-units=1`, `opt-level="z"` (now `opt-level=3` / `lto="fat"` / `codegen-units=1` / `strip=true`); dev: `debug=true`, dependencies `opt-level=2` |

> Note: at measurement time the `full` feature set was `http, mcp, streaming, timestamp, security, hot-reload, cache, websocket, grpc, logging, openapi` and did **not** include `cli` or `simd-json`; `full` has since grown to 26 runtime features and includes `cli` (see the main README's feature flags table). The compile-time/size numbers below correspond to the 2026-07-03 scope and do not map directly to the current `full` definition.

## 📦 Dependency Scale Comparison

| Metric | http only | full | Difference |
|------|-----------|------|------|
| `cargo tree` total lines | 862 | 1129 | +267 lines (+31%) |
| Deduplicated unique crates | 396 | 478 | +82 crates (+20.7%) |

Typical heavy dependencies introduced by `full`: `tonic`/`prost`/`tonic-prost` (gRPC), `tokio-tungstenite` (WebSocket), `rmcp` (MCP), `argon2`/`sha2`/`hmac`/`secrets` (security), `utoipa` (openapi), `oxcache` (cache), etc. None of these appear in the dependency graph of the `http only` build.

## ⏱️ Compile Time Comparison

Compile time uses cargo's reported value (`Finished ... in Xs`); wall-clock is the end-to-end duration including process startup/teardown.

| Feature combination | Debug compile time | Release compile time | Debug wall-clock | Release wall-clock |
|----------|---------------|-----------------|------------------|--------------------|
| http only | 28.88s | 13.72s | 30.07s | 17.11s |
| full | 54.52s | 25.48s | 59.98s | 28.81s |
| Ratio (full / http) | 1.89× | 1.86× | 2.00× | 1.69× |
| http savings vs full | **47.0%** | **46.2%** | 49.9% | 40.6% |

**Conclusion**: for both debug and release, `http only` saves about **46–47%** of compile time versus `full`. Debug mode saves about 25.6s; release mode saves about 11.8s.

## 💾 Binary / Artifact Size Comparison

### Framework library size (`libsdforge.rlib`, the cleanest framework-code metric)

| Feature combination | Debug rlib | Release rlib |
|----------|-----------|--------------|
| http only | 33.90 MB (35,540,214 B) | 3.57 MB (3,747,912 B) |
| full | 100.16 MB (105,027,940 B) | 9.07 MB (9,511,678 B) |
| Ratio (full / http) | 2.96× | 2.54× |
| http savings vs full | **66.2%** | **60.6%** |

### Total build footprint (`target/` directory)

| Feature combination | Debug deps | Debug target | Release deps | Release target |
|----------|-----------|--------------|--------------|----------------|
| http only | 808 MB | 1.1 GB | 575 MB | 608 MB |
| full | 1.5 GB | 2.0 GB | 792 MB | 832 MB |
| Ratio (full / http) | 1.86× | 1.82× | 1.38× | 1.37× |

> ⚠️ **Historical scope (2026-07-03)**: the following two sections were measured on a version where the repo still contained `src/main.rs` (a `panic` stub); that file was removed in v0.3.0 (2026-07-04, commit 3b92d73) and the repo currently has no root-package `[[bin]]` target, so `cargo build` no longer produces `target/release/sdforge` — the data and reproduction commands below are currently not executable and are kept only as a historical record.

### Standalone `sdforge` binary size

| Feature combination | Debug binary | Release binary |
|----------|-------------|----------------|
| http only | 3.73 MB (3,911,120 B) | 349 KB (357,296 B) |
| full | 3.73 MB (3,911,120 B, **identical**) | 349 KB (357,296 B, **identical**) |

> **Important note (historical scope)**: at measurement time the `full` feature set did not include `cli` (today's `full` includes `cli`), and `src/main.rs` degraded to the same `panic` stub in both scenarios (`#[cfg(not(feature = "cli"))] fn main()`); the file was later removed. This means **the default binary size cannot reflect the protocol-trimming difference** — evaluate via the `rlib` (still applicable today) or a real CLI binary.

### Real CLI binary size (adding the `cli` feature, release)

| Feature combination | Release binary |
|----------|----------------|
| http,cli | 5.76 MB (6,037,008 B) |
| full,cli | 5.90 MB (6,183,936 B) |
| Ratio (full / http) | 1.024× |
| http savings vs full | 2.4% |

> Even with `cli` enabled, the final binaries differ by only 2.4%: at measurement time the release profile (`lto=true` + `opt-level="z"` + `codegen-units=1`) aggressively dead-code-eliminated the binary, and protocol implementations unreferenced by `main.rs` were stripped at link time. **But this only affects the final binary size — the compile-time dependency cost still has to be paid** (see the compile-time table above).

## 🔍 Analysis

### 1. Why compile times differ

`full` pulls in 82 more unique crates than `http` (+20.7%); gRPC (tonic/prost), WebSocket (tokio-tungstenite), MCP (rmcp), security (argon2/sha2/hmac), and OpenAPI (utoipa) are all heavy, proc-macro-dense dependencies with large codegen. In debug mode full takes 89% longer (54.52s vs 28.88s) because debug keeps all symbols and debug info without optimization; in release the gap narrows slightly to 86% (25.48s vs 13.72s) — LTO and single codegen-unit slow both down, but full's extra dependency compile time remains. **The compile-time savings (about 46–47%) are the most direct and stable benefit of feature gating**, unaffected by final linker dead-code elimination.

### 2. Why artifact sizes differ

The `rlib` is the most accurate single metric of framework code size: in debug, full is 2.96× http (100.2 MB vs 33.9 MB); in release, 2.54× (9.07 MB vs 3.57 MB). The difference comes entirely from the trimmed protocol implementations and the dependency code inlined into the rlib. The total `target/` footprint grows from 1.1 GB to 2.0 GB in debug (+82%) and from 608 MB to 832 MB in release (+37%), which has real consequences for CI caches and disk budgets.

### 3. The measured effect of zero-cost abstraction

The data confirms "unused protocols produce zero compiled code": in the `http only` build, the source of MCP/WebSocket/gRPC/security modules appears neither in `cargo tree` (396 vs 478 crates) nor in `libsdforge.rlib` (33.9 MB vs 100.2 MB). This is not runtime checks or runtime loading but **complete compile-time trimming**: the code of disabled features is never processed by rustc. Quantified: debug saves 47% compile time and 66% library size; release saves 46% compile time and 61% library size.

### 4. Caveats on binary-size evaluation

At measurement time the default `sdforge` binary was identical under http and full (both panic stubs) and **cannot** serve as evidence of feature trimming; the repo now has no root-package binary target, so that dimension no longer applies. The real CLI binary (adding `cli`) differs by only 2.4% in release because the linker dead-code-eliminated unreferenced code. This reveals an important distinction:

- **Compile-time costs** (compile time, dependency graph, rlib size) are decided by the feature flags, and feature-gating benefits are significant;
- **Final binary size** is further shaped by LTO/dead-code elimination, so feature-gating's marginal benefit is partially diluted at the binary level — but only if you are willing to pay full compile time for `full`.

### 5. Impact on developer experience and CI/CD

- **Daily development**: the default `http only` full build takes 28.88s (debug), nearly twice as fast as `full`; incremental builds benefit even more (editing one file does not re-trigger gRPC/WebSocket and other unrelated dependencies).
- **CI matrix**: build per target protocol combination (e.g. `http` only, `http,websocket`, `mcp`) to shorten pipeline time and reduce cache pressure (target directory 37–82% smaller).
- **Artifact-size-sensitive scenarios** (embedded/edge/container images): release rlib drops from 9.07 MB to 3.57 MB, a positive gain for image layering and cold start.

## ✅ Conclusion

SDForge's compile-time feature gating achieves genuine "unused protocols produce zero compiled code", with the following quantified gains:

| Dimension | http only savings vs full |
|------|---------------------------|
| Compile time (debug) | 47.0% (28.88s vs 54.52s) |
| Compile time (release) | 46.2% (13.72s vs 25.48s) |
| Framework library rlib (debug) | 66.2% (33.9 MB vs 100.2 MB) |
| Framework library rlib (release) | 60.6% (3.57 MB vs 9.07 MB) |
| Total build footprint (debug target) | 45% (1.1 GB vs 2.0 GB) |
| Unique dependency crates | 17.2% (396 vs 478) |

Compared with the server-less (full bundling) approach, SDForge lets users who only need HTTP **compile 82 fewer crates, pay nearly half the compile time, and shrink library size by about 60%** — and the benefit happens at compile time with zero runtime overhead. Final binary size differs little under LTO dead-code elimination (CLI binary only 2.4%), but the compile-time and disk-footprint savings are certain and significant.

## 🔁 Reproduction

> Run from the SDForge repository root. `cargo clean` before each measurement to guarantee a from-zero build.

```bash
# Dependency scale
cargo tree --features http --prefix none | sort -u | wc -l   # 396
cargo tree --features full --prefix none | sort -u | wc -l   # 478

# === Debug compile time + rlib size ===
cargo clean && cargo build --features http 2>&1 | tail -1
ls -l target/debug/libsdforge.rlib

cargo clean && cargo build --features full 2>&1 | tail -1
ls -l target/debug/libsdforge.rlib

# === Release compile time + rlib size ===
cargo clean && cargo build --features http --release 2>&1 | tail -1
ls -l target/release/libsdforge.rlib

cargo clean && cargo build --features full --release 2>&1 | tail -1
ls -l target/release/libsdforge.rlib

# === Real CLI binary size (release; historical scope) ===
# The repo has no root-package [[bin]] target today (src/main.rs was removed in v0.3.0),
# so the following commands no longer produce target/release/sdforge; kept as a historical record.
cargo clean && cargo build --features http,cli --release 2>&1 | tail -1
ls -l target/release/sdforge

cargo clean && cargo build --features full,cli --release 2>&1 | tail -1
ls -l target/release/sdforge

# === Total build footprint ===
du -sh target/debug target/release
du -sh target/debug/deps target/release/deps
```

> Measurement date: 2026-07-03. Data is measured on the environment above; different machines/toolchain versions will produce different results, though the trends (http significantly faster than full, rlib significantly smaller than full) should hold. Note: both the release profile and the `full` feature set have since changed (see the notes above); the two binary-size sections rely on the then-existing `src/main.rs` entry point and are no longer reproducible.

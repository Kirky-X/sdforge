# 🧪 Sdforge Test Scenarios

This document pins down SDForge's test pyramid baseline, test target inventory, E2E scenario definitions, combination matrix, and static gates. All verification runs through `cargo test` (full scope `--workspace --all-features`).

## 📊 Test Pyramid Baseline

| Layer | Carrier | Count baseline |
|------|----------|----------|
| L1 lib unit tests | `#[cfg(test)]` in `src/**`: sdforge 2211 / sdforge-macros 85 / sdforge-examples 41 | 2337 passed |
| L2 integration tests | Main crate `tests/` 51 targets (integration/ 40 + unit/ 5 + top-level 4: integration_http_mcp, sdk_snapshots, mcp_stateful_macro, mcp_streamable_http + macros/macro_tests 1 + e2e 1: e2e_advanced, whose 157 count toward L3) + sdforge-macros crate `tests/` 2 targets (macro_compile/trybuild) + sdforge-examples crate `tests/` 3 targets (comprehensive_features, dbnexus_gateway_e2e, oxcache_admin_e2e) | 693 passed |
| L3 E2E scenarios | `tests/e2e/` (single target e2e_advanced, 11 domains) | 157 passed |
| L4 Doc-tests | sdforge 23 + sdforge-examples 5 | 28 passed + 45 ignored |

Full results (`--all-features --workspace`, measured 2026-11-17): **3215 passed / 0 failed / 73 ignored** (ignored = grpc_tests 28 environment-gated + Doc-tests 45 doc-example-gated).

## 🎯 Test Target Inventory

- Main crate: all 51 test targets explicitly registered via `[[test]]` (the tests/e2e, tests/integration, tests/unit, and tests/macros subdirectories are outside Cargo auto-discovery; unregistered targets vanish silently; the top-level integration_http_mcp is registered too; `test_target_inventory_tests` continuously asserts this line against drift in the integration/ file count)
- sdforge-macros / sdforge-examples crates: top-level `tests/` auto-discovery (macro_compile_tests, trybuild_tests, comprehensive_features, dbnexus_gateway_e2e, oxcache_admin_e2e)
- e2e_advanced: carried by the `tests/e2e/` directory (e2e_* files are never bare at the top level); subdirectories are outside auto-discovery and explicitly registered via `[[test]]`, preserving all 157 tests
- Feature gating: the 11 mods inside e2e_advanced each carry an independent `#[cfg(feature)]`, so any feature subset compiles (no required-features needed); the full scope `--all-features` carries no silent-skip risk

## 🧭 E2E Scenario Definitions (11 domains, 157 tests, 0 ignored)

tests/e2e/e2e_advanced.rs (scenario highlights taken from the public APIs declared by each mod):

| Domain | Count | Scenario highlights |
|----|------|----------|
| cache_advanced | 13 | DashMapCache / SyncCache advanced cache semantics |
| config_advanced | 19 | Config module advanced semantics |
| error_advanced | 23 | core::ServiceError classification / context / error chain |
| ratelimit_error_advanced | 8 | ApiError rate-limit error family mapping (exceeded/banned/circuit/quota/overflow saturation) |
| i18n_advanced | 26 | HttpI18nFormatter / parse_accept_language / I18nError multilingual fallback |
| security_advanced | 14 | JwtError / AuthError / AuthConfigError error families |
| ratelimit_advanced | 9 | RateLimiter / RateLimitError rate-limiting semantics |
| regex_cache_advanced | 15 | RegexCache / common / get_regex cache semantics |
| plugin_init_advanced | 4 | init_all_plugins full plugin initialization |
| cross_module_advanced | 24 | ApiError + LocalizedError + SdForgeError + TranslationStore cross-module interplay |
| validation_constants_advanced | 2 | core::validation validation constant boundaries |

## 📐 Combination Matrix

| Combination | Coverage | Result |
|------|------|------|
| `--all-features --workspace` (full scope) | All 61 producing targets | 3215 passed / 0 failed / 73 ignored |
| Seven-feature subset `http,security,cache,inklog,i18n,ratelimit,openapi` | e2e_advanced 11 domains | 157 passed / 0 failed |
| trybuild | Macro compile-failure diagnostic cases | 2 passed |

The 61 producing targets = main crate 51 + sdforge-macros tests 2 + sdforge-examples tests 3 + unittests 3 (sdforge/macros/examples) + Doc-tests 2 (sdforge/sdforge-examples; sdforge-macros Doc-tests produce no output and are not counted).

## 🚦 Static Gates

| Gate | Command scope | Result |
|------|----------|------|
| fmt | `cargo fmt --all -- --check` | Clean |
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Zero warnings |
| doc | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` | Zero warnings |
| deny | `cargo deny check` | 4 items ok (licenses clarify ×4) |
| audit | `cargo audit` | rc=0 (bincode migrated to postcard and out of the tree, RUSTSEC-2025-0141 no longer appears, exemption removed) |
| MSRV | rust-version (workspace unified to 1.97.1) | Consistent |

## ℹ️ Counting Scope and Ignored Items

- **Counting scope**: the full 3215 sums the `cargo test` output of each test target; empty targets with `0 passed; 0 failed` are not counted as producing targets.
- **grpc_tests 28 ignored**: upstream `#[ignore]` marks real network binding `127.0.0.1:0`, which hangs in CI / sandbox environments — environment gating, not a defect, and retained as such.
- **Doc-tests 45 ignored**: doc examples depend on their run context and are skipped by gating (sdforge 43 + sdforge-macros 2).
- **cargo-deny license verification**: four members (sdforge / sdforge-macros / limiteron / oxcache) declare licenses via license-file in `deny.toml` (a form cargo-deny does not recognize), pinned via clarify to MIT hash 0xfb13e7ad; the license-file path resolves relative to the clarified crate's manifest directory (sdforge-macros sits in the `macros/` subdirectory and uses `../LICENSE` to step up one level). Advisories / bans / sources checks all pass.

## 🧪 Test Strategy Matrix and Commands

### Test strategy matrix

<details>
<summary>📋 Table</summary>

| Layer | Location | Description |
|------|------|------|
| Unit tests | Embedded `#[cfg(test)]` in `src/`, `tests/unit/` | Module-level tests, including proptest property tests (`src/tests/property_tests.rs`) |
| Integration tests | `tests/integration/` | Covers http / mcp / grpc / websocket / security / cache / streaming / openapi / cli / docs / health_probes / graceful_shutdown / http_tls / validate / paginate / etag / lifecycle / otel_export / status_code / rbac protocol and feature combinations |
| Macro tests | `tests/macros/`, `macros/tests/` | trybuild compile-failure cases and macro expansion verification |
| E2E | `tests/e2e/` | `e2e_advanced` multi-domain E2E scenarios (11-domain scale baseline in the "E2E Scenario Definitions" section above) |
| Examples comprehensive tests | `examples/tests/` | All-feature re-export and cross-protocol dispatch (76 tests) plus gateway E2E |
| Benchmarks | `benches/`, `src/benches/` | criterion: `runtime_bench` / `config_and_cache_bench` / `sdforge_bench` |
| Doc-tests | `src/` doc comments | Embedded rustdoc examples |

</details>

### Running commands

<details>
<summary>📄 Code · bash</summary>

```bash
# Aligned with the CI matrix (http / mcp / http,mcp / http,security / http,cache /
# http,websocket / http,grpc / http,streaming / full — 9 combinations).
# The matrix tests the sdforge package only (-p sdforge): --workspace triggers
# feature unification through sdforge-examples' default features, which pulls
# nearly every sdforge feature into each combination.
cargo test -p sdforge --features "http,mcp"

# Full workspace (sdforge + sdforge-macros + sdforge-examples, all features)
cargo test --workspace --all-features

# Lib tests (the CI coverage target)
cargo test --features full --lib

# Coverage (CI gate >=90% line coverage: --features full --lib; lefthook pre-push applies the same threshold without --features full --lib)
cargo llvm-cov --features full --lib --lcov --fail-under-lines 90

# Formatting and zero-warning lint
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

</details>

The build.rs-generated protobuf code goes to `OUT_DIR` and no longer enters the source tree; the `src/grpc/pb/*` exclusion in `llvm-cov.toml` is a leftover from the old layout and currently matches no files.

### Test scale

About **3,273** test functions (`src/` 2,257 + `tests/` 800 + `macros/` 92 + `examples/` 124; via `grep -rEc '#\[(tokio::)?test\b' --include='*.rs'`, measured 2026-11-17).

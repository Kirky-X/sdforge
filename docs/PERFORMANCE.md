# ⚡ Sdforge 性能基线

> 基线记录日期：2026-09-11（路由/JSON）；TLS 组：2026-09-29
> 环境：WSL2 (linux 6.6.87) x64 · Rust 1.97.1 · release profile（`lto=fat`、`codegen-units=1`，仓库既有配置）
> criterion 报告为中位数区间 [low, median, high]，下表均取中位数。

复现命令：

```bash
cargo bench --bench runtime_bench --features http
cargo bench --bench runtime_bench --features serve-tls -- "tls_termination|plaintext_baseline|tls_cert_reloader"
cargo bench --bench runtime_bench --features grpc,streaming -- "grpc_stream"
```

> 本机基线数字用于回归参照（CI 阈值门禁待多机采样稳定后启用）。编译期成本（宏展开、依赖编译）单独记录于[编译期门控基准](benchmarks/vs-server-less.md)，不计入运行时基线。

## 📊 基线汇总

| 基准 | 覆盖路径 | 中位延迟 | 吞吐 |
|------|----------|----------|------|
| `plain_get` | 路由分发（无路径参数） | ~553 ns | ~1.81 M req/s |
| `path_param_get` | 路由分发（1 个路径参数） | ~604 ns | ~1.65 M req/s |
| `handler_args_build_5_params` | HandlerArgs 参数装配（5 参数） | ~117 ns | - |
| `serialize_nested_object` | JSON 序列化（7 字段嵌套对象） | ~137 ns | - |
| `deserialize_nested_object` | JSON 反序列化（同上对象） | ~383 ns | - |
| `https_request_pooled_conn` | TLS 复用连接请求往返（环回） | ~35 µs | ~28.9 K req/s |
| `https_handshake_and_request` | TLS 新建连接（握手+请求，环回） | ~559 µs | ~1.79 K req/s |
| `http_request_pooled_conn` | 明文复用连接请求往返（对照） | ~45 µs | ~22.4 K req/s |
| `resolve_read_path` | 证书重载器读路径（RwLock 读 + DER 访问） | ~48 ns | - |
| `reload_from_disk` | 证书热重载（读盘 + PEM 解析 + 装配） | ~26 µs | - |
| `reload_plus_4_concurrent_handshakes` | 1 次 reload 与 4 个并发新连接握手交叠 | ~3.26 ms | ~306 batch/s |
| `call_stream_10_items` | gRPC 流式分发（守卫链 + handler + 10 项收流） | ~7.3 µs | ~1.4 M items/s |
| `call_stream_100_items` | 同上（100 项） | ~45 µs | ~2.2 M items/s |
| `call_stream_1000_items` | 同上（1000 项） | ~352 µs | ~2.8 M items/s |
| `item_mapping_10k` | 每项序列化映射（stream_output_from，1e4 项） | ~1.52 ms | ~6.6 M items/s |
| `fallback_unregistered_key` | i18n 翻译回退（未注册键，纯锁+分配） | ~45 ns | ~22 M/s |
| `registry_hit_10k_entries` | i18n 翻译命中（1e4 注册项满表查表） | ~70 ns | ~14 M/s |

## 🔀 请求热路径（路由分发）

`route_dispatch/*` 从 `http::build()` 产物直接 `Service::call`，覆盖 axum 路由匹配 + `#[forge]` 生成的提取/序列化闭包（`plain_get` / `path_param_get` 延迟与吞吐见 [基线汇总](#-基线汇总)）。路径参数提取的额外开销约 **+50 ns/请求**（约 +9%）。

## 🔐 TLS 终止热路径（`serve-tls`）

`tls_termination/*` / `plaintext_baseline/*` / `tls_cert_reloader/*` 走真实环回 TCP（`serve_with_graceful_shutdown_tls` vs `serve_with_graceful_shutdown`，同 router 同排空配置，仅传输层不同）：

- **复用连接**：TLS 记录层加解密后请求往返 ~35 µs，与明文 ~45 µs 同数量级（环回噪声内）——keep-alive 连接上的持续开销可忽略。
- **新建连接**：含完整 TLS 1.3 握手的进入成本 ~559 µs（约为复用路径 16 倍），主要来自握手非对称操作与往返；短连接高 churn 部署按此估算容量。
- **证书热重载**：`resolve` 读路径 ~48 ns/次（握手路径上每次解析一次，`RwLock` 读无退化）；单次 `reload` ~26 µs（文件 IO + PEM 解析 + 密钥装配，tokio worker 上用 `reload_async`）；换盘与 4 路并发握手交叠的批次 ~3.26 ms——重载与进入流量互不阻塞。
- `resolve_read_path` 以 `end_entity_certificate()` 为代理口径（rustls `ClientHello` 无法在 bench 内构造，两者同为锁读 + 终端实体 DER 访问）。

## 🌊 gRPC server-streaming（`grpc` + `streaming`）

`grpc_stream/*` 直调 `SdForgeGrpcService::call_stream`（不经网络，聚焦分发+映射开销）：

- **增量线性**：10 项 ~7.3 µs → 1000 项 ~352 µs，每增量项约 ~350 ns（守卫链底价 + 每项映射）。
- **每项映射**：`stream_output_from` 消费 1e4 项 ~1.52 ms（约 152 ns/项，`to_value` + `extract_value` 双阶段）。
- 记录日期 2026-09-30（与 TLS 组同批）。

## 🌐 i18n 翻译查表

`i18n_translation/*`：`translate_or_fallback` 为控制面路径（MCP 工具列表 / CLI help / OpenAPI 每路由 / gRPC GetInfo），非请求热路径。回退 ~45 ns、满表（1e4 宿主注册项）命中 ~70 ns——锁开销线性、千级路由的 OpenAPI 生成（每路由 2 次临界区）总量微秒级，无回归风险。记录日期 2026-09-30。

## 🧩 HandlerArgs 参数装配

gRPC / CLI 统一处理入口共享的参数装配（`HandlerArgs` String map 构造），对应基线 `handler_args_build_5_params`（见 [基线汇总](#-基线汇总)）。

## 📦 JSON 序列化

对应基线 `serialize_nested_object` / `deserialize_nested_object`（7 字段嵌套对象，见 [基线汇总](#-基线汇总)）。`simd-json` feature 可进一步加速反序列化（尚未纳入本基线，后续轮次补充对比）。

## 🚧 基线外开销

- **宏展开编译期**：编译期成本见[编译期门控基准](benchmarks/vs-server-less.md)，`http` only 相比 `full` 省约 47% 编译时间；该口径明确不计入运行时基线。
- **全中间件栈（`build_with_config`）**：认证/限流/ETag/指标等按 feature 叠加，属部署配置函数，基线随特性组合浮动；按需后续增加分档基准。

## ✅ 回归门禁

- CI 性能阈值门禁暂未启用（多机噪声未标定）；以本文件为人工回归参照。
- 变更热路径代码时重跑上文复现命令，偏离中位数 ±15% 应在 PR 说明中给出理由。

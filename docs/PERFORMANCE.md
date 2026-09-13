# ⚡ Sdforge 性能基线

> 基线记录日期：2026-09-11
> 环境：WSL2 (linux 6.6.87) x64 · Rust 1.97.1 · release profile（`lto=fat`、`codegen-units=1`，仓库既有配置）
> criterion 报告为中位数区间 [low, median, high]，下表均取中位数。

复现命令：

```bash
cargo bench --bench runtime_bench --features http
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

## 🔀 请求热路径（路由分发）

`route_dispatch/*` 从 `http::build()` 产物直接 `Service::call`，覆盖 axum 路由匹配 + `#[forge]` 生成的提取/序列化闭包（`plain_get` / `path_param_get` 延迟与吞吐见 [基线汇总](#-基线汇总)）。路径参数提取的额外开销约 **+50 ns/请求**（约 +9%）。

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

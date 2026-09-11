# Performance Baselines — sdforge

> 基线记录日期：2026-09-11（T710，specmark: workspace-rc4-completion）
>
> 环境：WSL2 (linux 6.6.87) x64 · Rust 1.97.1 · release profile（`lto=fat`,
> `codegen-units=1`，仓库既有配置）
>
> 复现命令：
> ```text
> cargo bench --bench runtime_bench --features http
> ```
>
> 说明：本机基线数字用于回归参照（后续 CI 阈值待多机采样稳定后启用）。
> criterion 报告为中位数区间 [low, median, high]。

## 请求热路径（路由分发）

`route_dispatch/*` — 从 `http::build()` 产物直接 `Service::call`，覆盖
axum 路由匹配 + `#[forge]` 生成的提取/序列化闭包：

| 基准 | 中位延迟 | 吞吐 |
| --- | --- | --- |
| `plain_get`（无路径参数） | ~553 ns | ~1.81 M req/s |
| `path_param_get`（1 个 `{id}` 路径参数） | ~604 ns | ~1.65 M req/s |

路径参数提取的额外开销约 **+50 ns/请求**（约 +9%）。

## HandlerArgs（gRPC/CLI 统一处理入口的参数装配）

| 基准 | 中位延迟 |
| --- | --- |
| `handler_args_build_5_params`（5 参数 String map 构造） | ~117 ns |

## JSON 序列化

| 基准 | 中位延迟 |
| --- | --- |
| `serialize_nested_object`（7 字段嵌套对象） | ~137 ns |
| `deserialize_nested_object`（同上） | ~383 ns |

`simd-json` feature 可进一步加速反序列化（尚未纳入本基线，后续轮次补充对比）。

## 不在本基线内的开销

- **宏展开编译期**：编译期成本见 `docs/benchmarks/vs-server-less.md`
  （`http` only 相比 `full` 省约 47% 编译时间）。T710 口径明确不计入运行时基线。
- **全中间件栈（`build_with_config`）**：认证/限流/ETag/指标等按 feature 叠加，
  属部署配置函数，基线随特性组合浮动；按需后续增加分档基准。

## 回归门禁

- CI 性能阈值门禁暂未启用（多机噪声未标定）；以本文件为人工回归参照。
- 变更热路径代码时重跑上表命令，偏离中位数 ±15% 应在 PR 说明中给出理由。

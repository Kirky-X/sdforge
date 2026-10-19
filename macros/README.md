# sdforge-macros

[sdforge](https://github.com/Kirky-X/sdforge) 仓库的过程宏子 crate（目录 `sdforge/macros`）。

## 与父 crate 的关系

| 项 | 内容 |
|---|---|
| 所属仓库 | [sdforge](https://github.com/Kirky-X/sdforge)（[crates.io](https://crates.io/crates/sdforge) · [docs.rs](https://docs.rs/sdforge)） |
| 依赖方向 | `sdforge` → `sdforge-macros`（父 crate 以 `path + version` 双写依赖本 crate） |
| 是否 re-export | 是：父 crate 在 crate 根部导出 `forge`、`service_module`、`test_macro`；`log` 为宏展开内部使用的辅助宏，不单独导出 |
| 适用场景 | 通常无需直接依赖；父 crate 的 `macros` 能力即由本 crate 提供 |

## 提供的宏

| 宏 | 作用 |
|---|---|
| `#[forge]` | 标注一次函数即生成各协议注册代码（HTTP / MCP / gRPC / WebSocket / CLI），按启用的 feature 裁剪 |
| `#[service_module]` | 声明服务模块，聚合模块内的端点与元数据 |
| `#[test_macro]` | 生成测试辅助代码 |
| `apply_prefix` | 供宏展开调用的辅助过程宏 |

## 使用

经父 crate 使用（推荐）：

```rust
use sdforge::forge;

#[forge(name = "echo", version = "1.0", description = "Echo a greeting", cli = true)]
async fn echo(name: String) -> String {
    format!("Hello, {name}!")
}
```

## 版本与支持

- 版本经 `version.workspace = true` 随仓库统一，与 `sdforge` 同号发布。
- 仅维护当前 minor 版本线，详见 [sdforge 安全文档 · 支持的版本](https://github.com/Kirky-X/sdforge/blob/main/docs/SECURITY.md#-支持的版本与维护策略)。

## 许可证

MIT

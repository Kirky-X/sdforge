// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! SDForge CLI 基础示例。
//!
//! 演示如何用 `#[forge(cli = true)]` 定义命令，用 `CliBuilder::execute()`
//! 一站式构建 CLI、解析子命令并调用对应 handler。`execute()` 内部完成：
//! 1. `build()` 收集 inventory 中的 `CliCommandRegistration` 生成 `clap::Command`
//! 2. `get_matches()` 解析命令行
//! 3. `dispatch(&matches, state)` 路由到对应 `CliHandlerRegistration`
//! 4. `extract_value(&ret)` 智能提取返回值（`Value::String` → 原始串；其他 → JSON）
//! 5. 打印到 stdout / stderr 并 `std::process::exit(0/1)`
//!
//! 调用方只需 `#[tokio::main] async fn main() { cli.execute().await }`。
//!
//! ## 运行
//!
//! ```sh
//! # 查看帮助
//! cargo run --example basic_cli --features cli -- --help
//!
//! # echo 命令（--name 为必需参数）—— 输出 `Hello, world!`（无引号，验证 H3 智能提取）
//! cargo run --example basic_cli --features cli -- echo --name world
//!
//! # greet 命令（--greeting 为必需参数）
//! cargo run --example basic_cli --features cli -- greet --greeting "Hi"
//! ```

#![cfg(feature = "cli")]

use sdforge::cli::CliBuilder;
use sdforge::core::ApiError;
use sdforge::forge;

/// Echo a greeting. `name` 归为 Body 参数 → `--name <VALUE>` 选项（required）。
#[forge(
    name = "echo",
    version = "1.0",
    description = "Echo a greeting",
    cli = true
)]
async fn echo(name: String) -> Result<String, ApiError> {
    Ok(format!("Hello, {}!", name))
}

/// Custom greeting. `greeting` 归为 Body 参数 → `--greeting <VALUE>` 选项。
#[forge(
    name = "greet",
    version = "1.0",
    description = "Custom greeting",
    cli = true
)]
async fn greet(greeting: String) -> Result<String, ApiError> {
    Ok(format!("{} from sdforge!", greeting))
}

#[tokio::main]
async fn main() {
    // init_all_plugins 触碰 inventory 注册，防止链接器优化掉 CLI 命令注册项。
    sdforge::init_all_plugins();

    // CliBuilder::execute() 一站式完成 build / get_matches / dispatch / 输出 / 退出。
    // 返回类型为 `!`（never），因此之后不会有代码执行。
    CliBuilder::new().execute().await;
}

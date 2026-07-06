// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! SDForge CLI 基础示例。
//!
//! 演示如何用 `#[service_api(cli = true)]` 定义命令，用 `CliBuilder` 构建
//! CLI，解析子命令参数并调用对应 handler。
//!
//! ## 运行
//!
//! ```sh
//! # 查看帮助
//! cargo run --example basic_cli --features cli -- --help
//!
//! # echo 命令（--name 为必需参数）
//! cargo run --example basic_cli --features cli -- echo --name world
//!
//! # greet 命令（--greeting 为必需参数）
//! cargo run --example basic_cli --features cli -- greet --greeting "Hi"
//! ```

#![cfg(feature = "cli")]

use sdforge::cli::CliBuilder;
use sdforge::core::ApiError;
use sdforge::service_api;

/// Echo a greeting. `name` 归为 Body 参数 → `--name <VALUE>` 选项（required）。
#[service_api(
    name = "echo",
    version = "1.0",
    description = "Echo a greeting",
    cli = true
)]
async fn echo(name: String) -> Result<String, ApiError> {
    Ok(format!("Hello, {}!", name))
}

/// Custom greeting. `greeting` 归为 Body 参数 → `--greeting <VALUE>` 选项。
#[service_api(
    name = "greet",
    version = "1.0",
    description = "Custom greeting",
    cli = true
)]
async fn greet(greeting: String) -> Result<String, ApiError> {
    Ok(format!("{} from sdforge!", greeting))
}

fn main() {
    // init_all_plugins 触碰 inventory 注册，防止链接器优化掉 CLI 命令注册项。
    sdforge::init_all_plugins();

    // CliBuilder::build() 从 inventory 收集 CliCommandRegistration 构造 clap::Command。
    let mut cmd = CliBuilder::new().build();
    // get_matches(self) 消费 cmd，先缓存 help 文本供无子命令时打印。
    let help_text = cmd.render_help().to_string();
    let matches = cmd.get_matches();

    // 分发子命令。此处直接调用原始 async 函数打印返回值；
    // 生产场景可通过 inventory::iter::<CliHandlerRegistration> 查找 handler。
    match matches.subcommand() {
        Some(("echo", sub_matches)) => {
            let name = sub_matches
                .get_one::<String>("name")
                .map(|s| s.as_str())
                .unwrap_or("world");
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            match rt.block_on(echo(name.to_string())) {
                Ok(out) => println!("{}", out),
                Err(e) => eprintln!("error: {}", e),
            }
        }
        Some(("greet", sub_matches)) => {
            let greeting = sub_matches
                .get_one::<String>("greeting")
                .map(|s| s.as_str())
                .unwrap_or("Hi");
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            match rt.block_on(greet(greeting.to_string())) {
                Ok(out) => println!("{}", out),
                Err(e) => eprintln!("error: {}", e),
            }
        }
        _ => {
            // 无子命令或未知子命令时打印 help。
            println!("{}", help_text);
        }
    }
}

// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! CLI 端到端集成测试。
//!
//! 验证 `#[forge(cli = true)]` 宏生成的 `CliCommandRegistration` /
//! `CliHandlerRegistration` 能被 `CliBuilder::new().build()` 收集并构造为
//! `clap::Command`，子命令出现在 `--help` 输出中，参数按 Path/Body 规则映射。
//!
//! Feature requirement: run with
//! `cargo test --features cli --no-default-features --test cli_tests`.

#![cfg(feature = "cli")]

use sdforge::cli::CliBuilder;
use sdforge::core::ApiError;
use sdforge::forge;

// ============================================================================
// Test fixtures: 用 `#[forge(cli = true)]` 标注的命令。
//
// 宏会 emit `inventory::submit!(CliCommandRegistration { ... })` +
// `inventory::submit!(CliHandlerRegistration { ... })`，CliBuilder::build()
// 从 inventory 收集这些注册项构造 clap::Command。
// ============================================================================

/// Echo a greeting. `name` 无对应 path 参数，归为 Body → `--name <VALUE>`。
#[forge(
    name = "cli_test_echo",
    version = "1.0",
    description = "Echo a greeting",
    cli = true
)]
async fn cli_test_echo(name: String) -> Result<String, ApiError> {
    Ok(format!("Hello, {}!", name))
}

/// 无参数命令，验证空 args 注册正确。
#[forge(name = "cli_test_ping", version = "1.0", cli = true)]
async fn cli_test_ping() -> Result<String, ApiError> {
    Ok("pong".to_string())
}

// ============================================================================
// Tests
// ============================================================================

/// `CliBuilder::new().build()` 必须把 `cli_test_echo` 注册为子命令，
/// 子命令的 about 描述与宏参数 `description` 一致。
#[test]
fn test_cli_builder_includes_echo_subcommand() {
    let cmd = CliBuilder::new().build();
    let echo = cmd
        .find_subcommand("cli_test_echo")
        .expect("cli_test_echo must be a subcommand after build()");
    let about = echo.get_about().map(|s| s.to_string()).unwrap_or_default();
    assert!(
        about.contains("Echo a greeting"),
        "echo subcommand about must contain description, got: {}",
        about
    );
}

/// `--help` 输出必须包含已注册的命令名，证明 CLI 集成端到端工作。
#[test]
fn test_render_help_contains_command_names() {
    let mut cmd = CliBuilder::new().build();
    let help = cmd.render_help().to_string();
    assert!(
        help.contains("cli_test_echo"),
        "help output must contain 'cli_test_echo' subcommand: {}",
        help
    );
    assert!(
        help.contains("cli_test_ping"),
        "help output must contain 'cli_test_ping' subcommand: {}",
        help
    );
}

/// 无参数命令 `cli_test_ping` 必须注册且 version 字段正确。
#[test]
fn test_cli_ping_registered_with_version() {
    let cmd = CliBuilder::new().build();
    let ping = cmd
        .find_subcommand("cli_test_ping")
        .expect("cli_test_ping must be a subcommand");
    assert_eq!(
        ping.get_version(),
        Some("1.0"),
        "cli_test_ping version must be '1.0'"
    );
}

/// `cli_test_echo` 子命令必须有 `--name` 选项（Body 参数映射）且为 required。
#[test]
fn test_echo_subcommand_has_name_option() {
    let cmd = CliBuilder::new().build();
    let echo = cmd
        .find_subcommand("cli_test_echo")
        .expect("cli_test_echo must be a subcommand");
    let name_arg = echo
        .get_arguments()
        .find(|a| a.get_id().as_str() == "name")
        .expect("echo must have a `name` argument");
    assert!(
        name_arg.get_long().is_some(),
        "name arg must have a --long flag (Body → option)"
    );
    assert!(
        name_arg.is_required_set(),
        "name arg must be required (String, not Option)"
    );
}

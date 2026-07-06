// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! CLI Markdown 生成测试。
//!
//! 对应任务：T016。

use crate::cli::CliCommandRegistration;
use crate::docs::cli_markdown::generate_cli_docs;

// 注册一个测试专用 CLI 命令，用于验证 generate_cli_docs 能从 inventory
// 收集到命令并输出到 Markdown。命令名 `test_cmd_for_docs` 是唯一的，
// 不会与其他测试或生产代码冲突。
inventory::submit!(CliCommandRegistration::new(
    "test_cmd_for_docs",
    "1.0",
    "Test command for docs generation",
    "test_handler_for_docs",
));

/// `generate_cli_docs()` 应返回包含已注册命令名的 Markdown 字符串。
#[test]
fn test_generate_cli_docs_contains_command_name() {
    let md = generate_cli_docs();
    assert!(!md.is_empty(), "CLI Markdown 文档不应为空");
    assert!(
        md.contains("test_cmd_for_docs"),
        "CLI Markdown 应包含命令名 `test_cmd_for_docs`，实际: {}",
        &md[..md.len().min(300)]
    );
}

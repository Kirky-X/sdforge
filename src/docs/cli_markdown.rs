// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! CLI 命令手册 Markdown 生成。
//!
//! 调用 [`crate::cli::CliBuilder`] 构建 `clap::Command`，再用
//! `clap_markdown::help_markdown_command` 转换为 Markdown 文档。

/// 生成 CLI 命令手册 Markdown 文档。
///
/// 从 `inventory` 注册的 `CliCommandRegistration` 构建 `clap::Command` 树，
/// 再用 `clap_markdown` 转换为 Markdown。每个注册的子命令都会出现在文档中。
pub fn generate_cli_docs() -> String {
    let cmd = crate::cli::CliBuilder::new().build();
    clap_markdown::help_markdown_command(&cmd)
}

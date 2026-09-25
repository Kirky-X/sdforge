// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! CLI 命令手册 Markdown 生成。
//!
//! 调用 [`crate::cli::CliBuilder`] 构建 `clap::Command`，再用
//! `clap_markdown::help_markdown_command` 转换为 Markdown 文档。

/// 从外部构建的 `clap::Command` 生成 CLI 命令手册 Markdown。
///
/// [`clap_markdown::help_markdown_command`] 的薄包装：消费方（如宿主
/// 应用）可用同一渲染路径文档化自己的命令树，而不限于 sdforge 注册的
/// 命令集。
pub fn generate_cli_docs_from_command(cmd: &clap::Command) -> String {
    clap_markdown::help_markdown_command(cmd)
}

/// 生成 CLI 命令手册 Markdown 文档。
///
/// 从 `inventory` 注册的 `CliCommandRegistration` 构建 `clap::Command` 树，
/// 委托 [`generate_cli_docs_from_command`] 转换为 Markdown。每个注册的子
/// 命令都会出现在文档中。
pub fn generate_cli_docs() -> String {
    let cmd = crate::cli::CliBuilder::new().build();
    generate_cli_docs_from_command(&cmd)
}

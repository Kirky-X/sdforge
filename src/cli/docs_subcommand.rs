// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `docs` 子命令处理 — T020。
//!
//! 仅当 `docs` feature 启用时编译（`docs` 隐式包含 `cli`）。
//! 提供 [`docs_subcommand_definition`] 用于在 [`crate::cli::CliBuilder`]
//! 中注册 `docs` 子命令，以及 [`docs_subcommand`] 用于解析 `clap::ArgMatches`
//! 后分发到 [`crate::docs::generate_docs`] 或 [`crate::docs::write_docs`]。

#![cfg(feature = "docs")]

use std::path::PathBuf;

use crate::core::ApiError;
use crate::docs::{generate_docs, write_docs, DocFormat};

/// `docs` 子命令支持的格式名称与 [`DocFormat`] 变体的映射。
///
/// `value_parser` 限定 clap 接受的字符串集合，这里用静态切片方便扩展。
const FORMAT_VALUES: &[&str] = &["openapi", "swagger", "cli-markdown", "mcp-markdown", "all"];

/// 构造 `docs` 子命令的 `clap::Command` 定义。
///
/// 子命令接受两个参数：
/// - `--format <VALUE>`：文档格式（见 [`FORMAT_VALUES`]，默认 `all`）
/// - `--output <PATH>`：输出文件路径（省略时输出到 stdout）
pub fn docs_subcommand_definition() -> clap::Command {
    clap::Command::new("docs")
        .about("Generate documentation files")
        .arg(
            clap::Arg::new("format")
                .long("format")
                .value_parser(clap::builder::PossibleValuesParser::new(FORMAT_VALUES))
                .default_value("all")
                .help("Documentation format"),
        )
        .arg(
            clap::Arg::new("output")
                .long("output")
                .value_parser(clap::value_parser!(PathBuf))
                .help("Output file path (stdout if omitted)"),
        )
}

/// 根据 `--format` 字符串解析出 [`DocFormat`]。
///
/// 输入已在 clap 层通过 `value_parser` 限定为合法值，但这里仍以
/// `unwrap_or(DocFormat::All)` 兜底以防未来扩展遗漏——clap 已保证
/// 不会传入非法字符串，因此兜底分支实际不可达。
fn parse_format(s: &str) -> DocFormat {
    match s {
        "openapi" => DocFormat::OpenApi,
        "swagger" => DocFormat::SwaggerUi,
        "cli-markdown" => DocFormat::CliMarkdown,
        "mcp-markdown" => DocFormat::McpMarkdown,
        "all" => DocFormat::All,
        // 不可达：clap value_parser 已限定输入集合。用 unreachable! 显性化失败，
        // 避免未来扩展 FORMAT_VALUES 时遗漏此 match 分支导致静默产出 All 错误文档。
        _ => unreachable!("clap value_parser 已限定输入集合，得到非法值: {}", s),
    }
}

/// 执行 `docs` 子命令。
///
/// 从 `matches` 提取 `--format` 与 `--output`：
/// - 当 `--output` 提供时，调用 [`write_docs`] 写入指定文件
/// - 当 `--output` 省略时，调用 [`generate_docs`] 并 `println!` 到 stdout
///
/// IO 错误转换为 [`ApiError::Internal`] 并保留 source 用于错误链。
#[allow(clippy::result_large_err)]
pub fn docs_subcommand(matches: &clap::ArgMatches) -> Result<(), ApiError> {
    let format_str = matches
        .get_one::<String>("format")
        .map(|s| s.as_str())
        .unwrap_or("all");
    let format = parse_format(format_str);

    match matches.get_one::<PathBuf>("output") {
        Some(path) => write_docs(format, path)
            .map_err(|e| ApiError::internal_with_source("docs write failed", "docs_subcommand", e)),
        None => {
            let content = generate_docs(format).map_err(|e| {
                ApiError::internal_with_source("docs generation failed", "docs_subcommand", e)
            })?;
            println!("{}", content);
            Ok(())
        }
    }
}

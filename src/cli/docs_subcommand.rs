// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `docs` 子命令处理。
//!
//! 仅当 `docs` feature 启用时编译（`docs` 隐式包含 `cli`）。
//! **`docs` 是保留子命令名**：dispatch 在查找用户注册前优先拦截该名字
//! 执行文档生成，下游经 `#[forge(cli = true)]` 注册同名命令将不可达。
//! 提供 [`docs_subcommand_definition`] 用于在 [`crate::cli::CliBuilder`]
//! 中注册 `docs` 子命令，以及 [`docs_subcommand`] 用于解析 `clap::ArgMatches`
//! 后分发到 [`crate::docs::generate_docs`] 或 [`crate::docs::write_docs`]。

use std::path::PathBuf;

use crate::core::ApiError;
use crate::docs::{DocFormat, generate_docs, write_docs};

/// `docs` 子命令支持的格式名称与 [`DocFormat`] 变体的映射（单一事实源：
/// clap 白名单、Agent 知识包的 docs 条目与一致性测试都从这里派生，
/// 新增文档格式只改此处与 `parse_format` 分支）。
pub(crate) const FORMAT_VALUES: [&str; 6] = [
    "openapi",
    "swagger",
    "cli-markdown",
    "mcp-markdown",
    "all",
    "agent",
];

/// `docs --format` 的默认取值（与 `parse_format` 的 `all` 分支对应）。
pub(crate) const DEFAULT_DOC_FORMAT: &str = "all";

/// 构造 `docs` 子命令的 `clap::Command` 定义。
///
/// 子命令接受两个参数：
/// - `--format <VALUE>`：文档格式（见 \[`FORMAT_VALUES`\]，默认 `all`）
/// - `--output <PATH>`：输出文件路径（省略时输出到 stdout）
pub fn docs_subcommand_definition() -> clap::Command {
    clap::Command::new("docs")
        .about("Generate documentation files")
        .arg(
            clap::Arg::new("format")
                .long("format")
                .value_parser(clap::builder::PossibleValuesParser::new(FORMAT_VALUES))
                .default_value(DEFAULT_DOC_FORMAT)
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
/// 用户输入已在 clap 层通过 `value_parser` 限定为 [`FORMAT_VALUES`] 集合。
/// 此外，内建全局 `--format text|json` 的值会从子命令 matches 穿透读取
/// （实测 clap 4.6：`prog --format json docs` 时 `sub.get_one("format")`
/// 返回 `"json"`，即便传播对同 id arg 跳过）——`text`/`json` 不是文档
/// 格式，显式回落 docs 自身默认 [`DocFormat::All`]，两个开关语义独立。
fn parse_format(s: &str) -> DocFormat {
    match s {
        "openapi" => DocFormat::OpenApi,
        "swagger" => DocFormat::SwaggerUi,
        "cli-markdown" => DocFormat::CliMarkdown,
        "mcp-markdown" => DocFormat::McpMarkdown,
        "all" => DocFormat::All,
        "agent" => DocFormat::Agent,
        // 全局 --format 的穿透值：非文档格式，按 docs 默认（All）处理。
        "text" | "json" => DocFormat::All,
        // clap value_parser 白名单之外的值不可达；出现即说明扩展
        // FORMAT_VALUES 时遗漏了 match 分支——loud fail 而非静默产 All。
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

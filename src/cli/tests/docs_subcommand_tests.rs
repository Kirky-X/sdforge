// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T020 测试：`docs` 子命令处理。
//!
//! 仅当 `docs` feature 启用时编译（`docs` 隐式包含 `cli`）。
//! 验证 `docs_subcommand_definition()` 返回的 `clap::Command` 含
//! `--format` 与 `--output` 参数，且 `docs_subcommand(&matches)` 能
//! 正确分发到 `write_docs` / `generate_docs`。

#![cfg(feature = "docs")]

use crate::cli::docs_subcommand::{docs_subcommand, docs_subcommand_definition};

/// `docs_subcommand_definition()` 返回的命令必须包含 `--format` 长选项。
#[test]
fn test_docs_subcommand_definition_has_format_arg() {
    let cmd = docs_subcommand_definition();
    let format_arg = cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "format")
        .expect("--format 参数必须存在");
    assert_eq!(
        format_arg.get_long(),
        Some("format"),
        "format 必须是 --long 选项"
    );
    // value_parser 限定的可选值集合通过 get_value_parser 暴露，
    // 但 clap 未提供枚举原始值的公开 API；这里间接验证 default 值
    // 是 "all"（验证子命令默认行为）。
    let defaults: Vec<String> = format_arg
        .get_default_values()
        .iter()
        .map(|v| v.to_string_lossy().into_owned())
        .collect();
    assert_eq!(defaults, vec!["all".to_string()], "format 默认值必须是 all");
}

/// `docs_subcommand_definition()` 返回的命令必须包含 `--output` 长选项。
#[test]
fn test_docs_subcommand_definition_has_output_arg() {
    let cmd = docs_subcommand_definition();
    let output_arg = cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "output")
        .expect("--output 参数必须存在");
    assert_eq!(
        output_arg.get_long(),
        Some("output"),
        "output 必须是 --long 选项"
    );
    // --output 未设置 default_value，必须非 required
    assert!(!output_arg.is_required_set(), "output 必须是可选的");
}

/// 当提供 `--output` 时，`docs_subcommand(&matches)` 必须将文档写入
/// 指定文件。使用 `tempfile::tempdir()` 获取隔离的临时目录，
/// 调用 `--format openapi --output <tmp>/spec.json`，断言文件存在且非空。
#[test]
fn test_docs_subcommand_writes_file() {
    let dir = tempfile::tempdir().expect("无法创建临时目录");
    let output_path = dir.path().join("spec.json");

    let cmd = docs_subcommand_definition();
    let matches = cmd
        .try_get_matches_from([
            "docs",
            "--format",
            "openapi",
            "--output",
            output_path.to_str().unwrap(),
        ])
        .expect("解析参数失败");

    docs_subcommand(&matches).expect("docs_subcommand 必须成功");

    // 文件必须存在
    let metadata = std::fs::metadata(&output_path).expect("输出文件必须存在");
    assert!(metadata.is_file(), "输出必须是普通文件");
    // 文件必须非空
    assert!(
        metadata.len() > 0,
        "输出文件必须非空（OpenAPI JSON 应有内容）"
    );

    // 验证内容是合法 JSON（OpenApi 变体产出 JSON）
    let content = std::fs::read_to_string(&output_path).expect("读取输出文件失败");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("OpenAPI 输出必须是合法 JSON");
    // OpenAPI 文档必须包含 openapi 版本字段
    assert!(
        parsed.get("openapi").is_some(),
        "OpenAPI 输出必须包含 `openapi` 字段"
    );
}

/// 当未提供 `--output` 时，`docs_subcommand(&matches)` 必须返回 `Ok(())`，
/// 通过 stdout 输出文档（此处不验证 stdout 内容以避免测试脆弱）。
#[test]
fn test_docs_subcommand_stdout_when_no_output() {
    let cmd = docs_subcommand_definition();
    let matches = cmd
        .try_get_matches_from(["docs", "--format", "cli-md"])
        .expect("解析参数失败");

    // 必须返回 Ok(()) —— 不验证 stdout 内容，仅验证调用路径无误
    let result = docs_subcommand(&matches);
    assert!(result.is_ok(), "无 --output 时 docs_subcommand 必须返回 Ok");
}

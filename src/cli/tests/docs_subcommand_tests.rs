// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 测试：`docs` 子命令处理。
//!
//! 仅当 `docgen` feature 启用时编译（`docgen` 隐式包含 `cli`）。
//! 验证 `docs_subcommand_definition()` 返回的 `clap::Command` 含
//! `--format` 与 `--output` 参数，且 `docs_subcommand(&matches)` 能
//! 正确分发到 `write_docs` / `generate_docs`。

use crate::cli::{docs_subcommand, docs_subcommand_definition};

/// `docs_subcommand_definition()` 返回的命令必须包含 `--format` 长选项。
#[test]
fn test_docs_subcommand_definition_has_format_arg() {
    let cmd = docs_subcommand_definition();
    let format_arg = cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "format")
        .expect("the --format argument must exist");
    assert_eq!(
        format_arg.get_long(),
        Some("format"),
        "format must be a --long option"
    );
    // value_parser 限定的可选值集合通过 get_value_parser 暴露，
    // 但 clap 未提供枚举原始值的公开 API；这里间接验证 default 值
    // 是 "all"（验证子命令默认行为）。
    let defaults: Vec<String> = format_arg
        .get_default_values()
        .iter()
        .map(|v| v.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        defaults,
        vec!["all".to_string()],
        "the format default must be all"
    );
}

/// `docs_subcommand_definition()` 返回的命令必须包含 `--output` 长选项。
#[test]
fn test_docs_subcommand_definition_has_output_arg() {
    let cmd = docs_subcommand_definition();
    let output_arg = cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "output")
        .expect("the --output argument must exist");
    assert_eq!(
        output_arg.get_long(),
        Some("output"),
        "output must be a --long option"
    );
    // --output sets no default_value, so it must not be required
    assert!(!output_arg.is_required_set(), "output must be optional");
}

/// 当提供 `--output` 时，`docs_subcommand(&matches)` 必须将文档写入
/// 指定文件。使用 `tempfile::tempdir()` 获取隔离的临时目录，
/// 调用 `--format openapi --output <tmp>/spec.json`，断言文件存在且非空。
#[test]
fn test_docs_subcommand_writes_file() {
    let dir = tempfile::tempdir().expect("create temp dir");
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
        .expect("arg parsing failed");

    docs_subcommand(&matches).expect("docs_subcommand must succeed");

    // The file must exist
    let metadata = std::fs::metadata(&output_path).expect("the output file must exist");
    assert!(metadata.is_file(), "the output must be a regular file");
    // The file must be non-empty
    assert!(
        metadata.len() > 0,
        "the output file must be non-empty (OpenAPI JSON should have content)"
    );

    // Verify the content is valid JSON (the OpenApi variant produces JSON)
    let content = std::fs::read_to_string(&output_path).expect("read the output file");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("the OpenAPI output must be valid JSON");
    // The OpenAPI document must contain the openapi version field
    assert!(
        parsed.get("openapi").is_some(),
        "the OpenAPI output must contain the `openapi` field"
    );
}

/// 当未提供 `--output` 时，`docs_subcommand(&matches)` 必须返回 `Ok(())`，
/// 通过 stdout 输出文档（此处不验证 stdout 内容以避免测试脆弱）。
#[test]
fn test_docs_subcommand_stdout_when_no_output() {
    let cmd = docs_subcommand_definition();
    let matches = cmd
        .try_get_matches_from(["docs", "--format", "cli-markdown"])
        .expect("arg parsing failed");

    // Must return Ok(()) — stdout content is not verified here
    let result = docs_subcommand(&matches);
    assert!(
        result.is_ok(),
        "docs_subcommand must return Ok without --output"
    );
}

/// `--format agent` 必须是 docs 子命令的合法取值（Agent 知识包输出），
/// 且能走通 docs_subcommand 的生成路径。
#[test]
fn test_docs_subcommand_accepts_agent_format() {
    let cmd = docs_subcommand_definition();
    let matches = cmd
        .try_get_matches_from(["docs", "--format", "agent"])
        .expect("agent must be an accepted --format value");

    let result = docs_subcommand(&matches);
    assert!(result.is_ok(), "agent format must generate without error");
}

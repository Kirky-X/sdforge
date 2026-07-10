// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Docs 端到端集成测试。
//!
//! 验证 `generate_docs(All)` 输出包含 OpenAPI JSON + CLI Markdown + MCP
//! Markdown，以及 `write_docs` 文件写入 roundtrip。
//!
//! Feature requirement: run with
//! `cargo test --features docs --no-default-features --test docs_tests`.

#![cfg(feature = "docs")]

use sdforge::core::ApiError;
use sdforge::service_api;
use sdforge::{DocFormat, generate_docs, write_docs};

// ============================================================================
// Test fixture: 注册测试用 CLI 命令（docs 隐式包含 cli feature）。
// generate_docs(CliMarkdown) 从 inventory 收集 CliCommandRegistration。
// ============================================================================

#[service_api(
    name = "docs_test_cmd",
    version = "1.0",
    description = "Test command for docs generation",
    cli = true
)]
async fn docs_test_cmd(name: String) -> Result<String, ApiError> {
    Ok(format!("docs:{}", name))
}

// ============================================================================
// Tests
// ============================================================================

/// `generate_docs(OpenApi)` 必须返回 OpenAPI 3.1 JSON（以 `{` 开头）。
#[test]
fn test_generate_docs_openapi_returns_json() {
    let json = generate_docs(DocFormat::OpenApi).expect("generate_docs OpenApi");
    assert!(
        json.starts_with('{'),
        "OpenAPI output should be JSON object, got: {}",
        &json[..json.len().min(200)]
    );
}

/// `generate_docs(CliMarkdown)` 必须返回非空 Markdown 且包含已注册命令名。
#[test]
fn test_generate_docs_cli_markdown_contains_command() {
    let md = generate_docs(DocFormat::CliMarkdown).expect("generate_docs CliMarkdown");
    assert!(!md.is_empty(), "CLI markdown should not be empty");
    assert!(
        md.contains("docs_test_cmd"),
        "CLI markdown should contain 'docs_test_cmd' command: {}",
        &md[..md.len().min(300)]
    );
}

/// `generate_docs(All)` 拼接 OpenApi + CliMarkdown + McpMarkdown，必须包含
/// JSON 标记和已注册 CLI 命令名。
#[test]
fn test_generate_docs_all_combines_formats() {
    let all = generate_docs(DocFormat::All).expect("generate_docs All");
    assert!(
        all.contains('{') || all.contains('#'),
        "All format should contain JSON '{{' or markdown '#' markers: {}",
        &all[..all.len().min(200)]
    );
    assert!(
        all.contains("docs_test_cmd"),
        "All format should contain CLI command name: {}",
        &all[..all.len().min(400)]
    );
}

/// `write_docs` 写入文件 roundtrip：文件存在、内容非空、为 JSON。
#[test]
fn test_write_docs_to_file_roundtrip() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("docs.json");
    write_docs(DocFormat::OpenApi, &path).expect("write_docs should succeed");
    assert!(path.exists(), "output file should exist");
    let content = std::fs::read_to_string(&path).expect("read file");
    assert!(!content.is_empty(), "file content should not be empty");
    assert!(
        content.starts_with('{'),
        "file content should be JSON: {}",
        &content[..content.len().min(200)]
    );
}

/// MCP Markdown 在 mcp feature 启用时返回工具列表；未启用时返回占位提示。
/// 两种情况都不得 panic 且返回非空字符串。
#[test]
fn test_generate_docs_mcp_markdown_returns_nonempty() {
    let md = generate_docs(DocFormat::McpMarkdown).expect("generate_docs McpMarkdown");
    assert!(!md.is_empty(), "MCP markdown should not be empty");
}

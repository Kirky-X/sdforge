// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `docs` 模块主测试 — `DocFormat` 枚举、`generate_docs`、`write_docs`。
//!
//! 对应任务：T011 / T012 / T013 / T015 / T017 / T019。

use crate::docs::{generate_docs, write_docs, DocFormat};

// ============================================================================
// T011: DocFormat 枚举
// ============================================================================

/// 验证 `DocFormat` 5 个变体存在、互异，且派生了 `Debug`/`Clone`/`Copy`/
/// `PartialEq`/`Eq`/`Default`（默认 `OpenApi`）。
#[test]
fn test_doc_format_variants() {
    let openapi = DocFormat::OpenApi;
    let swagger = DocFormat::SwaggerUi;
    let cli_md = DocFormat::CliMarkdown;
    let mcp_md = DocFormat::McpMarkdown;
    let all = DocFormat::All;

    // 变体互异性 (PartialEq + Eq)
    let variants = [openapi, swagger, cli_md, mcp_md, all];
    for (i, &a) in variants.iter().enumerate() {
        for (j, &b) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(a, b, "相同变体必须相等: idx={}", i);
            } else {
                assert_ne!(a, b, "不同变体必须不等: idx={} vs {}", i, j);
            }
        }
    }

    // Debug 派生
    let debug = format!("{:?}", openapi);
    assert!(
        debug.contains("OpenApi"),
        "Debug 输出应含变体名，实际: {}",
        debug
    );

    // Clone 派生
    let cloned = openapi.clone();
    assert_eq!(openapi, cloned);

    // Copy 派生 — 赋值后原值仍可用
    let copied = openapi;
    assert_eq!(openapi, copied);

    // Default 派生 — 默认 OpenApi
    let default = DocFormat::default();
    assert_eq!(default, DocFormat::OpenApi, "Default 应为 OpenApi");
}

// ============================================================================
// T012: generate_docs 入口
// ============================================================================

/// `generate_docs(OpenApi)` 应返回非空 JSON 字符串，首字符为 `{`。
#[test]
fn test_generate_docs_openapi_returns_json() {
    let json = generate_docs(DocFormat::OpenApi);
    assert!(!json.is_empty(), "OpenApi 文档不应为空");
    assert!(
        json.trim_start().starts_with('{'),
        "OpenApi 文档首字符应为 `{{`，实际首字符: {:?}",
        json.chars().next()
    );
}

/// `generate_docs(CliMarkdown)` 应返回含 markdown 标记（`#` 或 `##`）的非空文本。
#[test]
fn test_generate_docs_cli_markdown_returns_md() {
    let md = generate_docs(DocFormat::CliMarkdown);
    assert!(!md.is_empty(), "CliMarkdown 文档不应为空");
    assert!(
        md.contains('#'),
        "CliMarkdown 文档应含 markdown 标题标记 `#`，实际: {}",
        &md[..md.len().min(200)]
    );
}

// ============================================================================
// T013: write_docs 写文件
// ============================================================================

/// `write_docs` 应将生成的文档写入指定路径，文件存在且内容非空。
#[test]
fn test_write_docs_creates_file() {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let file_path = dir.path().join("openapi.json");

    write_docs(DocFormat::OpenApi, &file_path).expect("write_docs 应成功");

    // 文件存在
    assert!(file_path.exists(), "文档文件应被创建: {:?}", file_path);

    // 内容非空
    let content = std::fs::read_to_string(&file_path).expect("读取写入的文件");
    assert!(!content.is_empty(), "文档内容不应为空");
    assert!(
        content.trim_start().starts_with('{'),
        "OpenApi 文档首字符应为 `{{`，实际: {:?}",
        content.chars().next()
    );
}

// ============================================================================
// T015: generate_docs 支持 SwaggerUi
// ============================================================================

/// `generate_docs(SwaggerUi)` 应返回含 `<html` 和 `swagger-ui` 的 HTML 字符串。
#[test]
fn test_generate_docs_swagger_returns_html() {
    let html = generate_docs(DocFormat::SwaggerUi);
    assert!(!html.is_empty(), "SwaggerUi HTML 不应为空");
    let lower = html.to_lowercase();
    assert!(
        lower.contains("<html"),
        "SwaggerUi 输出应含 <html 标签，实际: {}",
        &html[..html.len().min(200)]
    );
    assert!(
        lower.contains("swagger-ui"),
        "SwaggerUi 输出应含 swagger-ui 链接，实际: {}",
        &html[..html.len().min(200)]
    );
}

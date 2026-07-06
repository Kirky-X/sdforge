// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `docs` 模块主测试 — `DocFormat` 枚举、`generate_docs`、`write_docs`。
//!
//! 对应任务：T011 / T012 / T013 / T015 / T017 / T019。

use crate::docs::DocFormat;

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

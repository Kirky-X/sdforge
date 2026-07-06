// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! MCP Markdown 生成测试。
//!
//! 对应任务：T018。整个文件需 `mcp` feature 门控。

#![cfg(feature = "mcp")]

use crate::core::ApiMetadata;
use crate::docs::mcp_markdown::generate_mcp_docs;
use crate::mcp::{McpToolRegistration, SdForgeTool};
use rmcp::model::{CallToolResult, ContentBlock, ErrorData as McpError};
use serde_json::Value;
use std::sync::Arc;

// 注册一个测试专用 MCP 工具，用于验证 generate_mcp_docs 能从 inventory
// 收集到工具并输出到 Markdown。工具名 `test_mcp_tool_for_docs` 是唯一的。
fn create_test_mcp_tool_for_docs() -> Arc<dyn SdForgeTool> {
    struct TestMcpToolForDocs;
    impl SdForgeTool for TestMcpToolForDocs {
        fn name(&self) -> &str {
            "test_mcp_tool_for_docs"
        }
        fn description(&self) -> &str {
            "A test MCP tool for docs generation"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
        fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
            Ok(CallToolResult::success(vec![ContentBlock::text(
                "ok".to_string(),
            )]))
        }
    }
    Arc::new(TestMcpToolForDocs) as Arc<dyn SdForgeTool>
}

fn create_test_mcp_metadata_for_docs() -> ApiMetadata {
    ApiMetadata {
        name: "test_mcp_tool_for_docs".to_string(),
        version: "v1".to_string(),
        description: "A test MCP tool for docs generation".to_string(),
        cache_ttl: None,
        is_streaming: false,
    }
}

inventory::submit!(McpToolRegistration::new(
    "test_mcp_tool_for_docs",
    "v1",
    create_test_mcp_tool_for_docs,
    create_test_mcp_metadata_for_docs,
));

/// `generate_mcp_docs()` 应返回包含已注册工具名的 Markdown 字符串。
#[test]
fn test_generate_mcp_docs_contains_tool_name() {
    let md = generate_mcp_docs();
    assert!(!md.is_empty(), "MCP Markdown 文档不应为空");
    assert!(
        md.contains("test_mcp_tool_for_docs"),
        "MCP Markdown 应包含工具名 `test_mcp_tool_for_docs`，实际: {}",
        &md[..md.len().min(300)]
    );
    assert!(
        md.contains("MCP Tools"),
        "MCP Markdown 应含 `# MCP Tools` 标题"
    );
}

/// `generate_mcp_docs()` 应包含工具的描述（从 metadata 获取）。
#[test]
fn test_generate_mcp_docs_contains_description() {
    let md = generate_mcp_docs();
    assert!(
        md.contains("A test MCP tool for docs generation"),
        "MCP Markdown 应含工具描述"
    );
}

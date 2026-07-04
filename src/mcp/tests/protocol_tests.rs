// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for MCP protocol types: `CallToolResult`, `Content`, `ErrorData` (McpError).

use rmcp::model::{CallToolResult, ContentBlock, ErrorData as McpError, Meta};

#[test]
fn test_content_text_helper() {
    let content = ContentBlock::text("hello".to_string());
    // Verify content was created (the exact field access depends on rmcp's API)
    assert!(format!("{:?}", content).contains("hello"));
}

#[test]
fn test_call_tool_result_success() {
    let result = CallToolResult::success(vec![ContentBlock::text("test".to_string())]);
    assert_eq!(result.is_error, Some(false));
    assert!(!result.content.is_empty());
}

#[test]
fn test_call_tool_result_error() {
    let result = CallToolResult::error(vec![ContentBlock::text("error".to_string())]);
    assert_eq!(result.is_error, Some(true));
}

#[test]
fn test_call_tool_result_with_meta() {
    let mut result = CallToolResult::success(vec![]);
    result.is_error = None;
    result.meta = Some(Meta::default());
    assert!(result.meta.is_some());
}

#[test]
fn test_call_tool_result_with_structured_content() {
    let mut result = CallToolResult::success(vec![]);
    result.is_error = None;
    result.structured_content = Some(serde_json::json!({"result": "ok"}));
    assert!(result.structured_content.is_some());
    assert_eq!(result.structured_content.unwrap()["result"], "ok");
}

#[test]
fn test_mcp_error_invalid_params() {
    let err = McpError::invalid_params("test error", None);
    assert!(err.message.contains("test error"));
}

#[test]
fn test_mcp_error_method_not_found() {
    // rmcp 0.16's method_not_found() takes a ConstString type parameter,
    // which is awkward to use directly. Use ErrorData::new with the
    // METHOD_NOT_FOUND error code instead for this test.
    use rmcp::model::ErrorCode;
    let err = McpError::new(ErrorCode::METHOD_NOT_FOUND, "unknown_method", None);
    assert!(format!("{:?}", err).contains("unknown_method"));
}

#[test]
fn test_mcp_error_internal_error() {
    let err = McpError::internal_error("internal failure", None);
    assert!(err.message.contains("internal failure"));
}

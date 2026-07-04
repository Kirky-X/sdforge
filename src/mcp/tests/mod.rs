// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! MCP module test suites.
//!
//! Tests are organized by responsibility:
//! - `server_tests`: `SdForgeMcpServer` construction, lookup, `ServerHandler` integration
//! - `handler_tests`: `SdForgeTool` trait implementations and tool execution behavior
//! - `protocol_tests`: `CallToolResult`, `Content`, `ErrorData` (McpError) protocol types
//! - `migration_tests`: `McpToolRegistration`, `get_mcp_tools`, inventory collection

mod handler_tests;
mod migration_tests;
mod protocol_tests;
mod server_tests;

use crate::core::ApiMetadata;
use crate::mcp::{McpToolRegistration, SdForgeTool};
use rmcp::model::{CallToolResult, ContentBlock, ErrorData as McpError};
use serde_json::Value;
use std::sync::Arc;

// ============================================================================
// Shared test helpers — accessible by all sub-modules via `super::`
// ============================================================================

/// Exercise all `SdForgeTool` trait methods for coverage.
pub(super) fn exercise_all_tool_methods(tool: &dyn SdForgeTool) {
    let _ = tool.name();
    let _ = tool.description();
    let _ = tool.input_schema();
    let _ = tool.call(None);
}

/// Create a simple test tool named "test".
pub(super) fn create_test_tool() -> Arc<dyn SdForgeTool> {
    struct TestTool;
    impl SdForgeTool for TestTool {
        fn name(&self) -> &str {
            "test"
        }
        fn description(&self) -> &str {
            "Test tool"
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
    Arc::new(TestTool) as Arc<dyn SdForgeTool>
}

/// Create test metadata for the `test` tool.
pub(super) fn create_test_metadata() -> ApiMetadata {
    ApiMetadata {
        name: "test_tool".to_string(),
        version: "v1".to_string(),
        description: "A test tool".to_string(),
        cache_ttl: None,
        is_streaming: false,
    }
}

/// Globally-registered coverage test tool (registered via inventory at compile time).
fn create_coverage_test_tool() -> Arc<dyn SdForgeTool> {
    struct CoverageTestTool;
    impl SdForgeTool for CoverageTestTool {
        fn name(&self) -> &str {
            "coverage_test_tool"
        }
        fn description(&self) -> &str {
            "A tool registered for inventory coverage"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
        fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
            Ok({
                let mut result = CallToolResult::success(vec![]);
                result.is_error = None;
                result
            })
        }
    }
    Arc::new(CoverageTestTool) as Arc<dyn SdForgeTool>
}

fn create_coverage_test_metadata() -> ApiMetadata {
    ApiMetadata {
        name: "coverage_test_tool".to_string(),
        version: "v1".to_string(),
        description: "A tool registered for inventory coverage".to_string(),
        cache_ttl: None,
        is_streaming: false,
    }
}

// Register the coverage tool in the global inventory so `get_mcp_tools()` has at
// least one entry in test builds.
inventory::submit!(McpToolRegistration::new(
    "coverage_test_tool",
    "v1",
    create_coverage_test_tool,
    create_coverage_test_metadata,
));

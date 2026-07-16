// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `SdForgeMcpServer`: construction, lookup, and `ServerHandler` integration.

use super::{create_test_metadata, create_test_tool};
use crate::mcp::{McpToolInstance, SdForgeMcpServer, build};
use rmcp::handler::server::ServerHandler;

#[test]
fn test_build_returns_server_with_tools() {
    let server = build();
    assert!(
        server.tool_count() > 0,
        "server should have registered tools"
    );
}

#[test]
fn test_server_find_tool() {
    let server = build();
    assert!(server.find_tool("coverage_test_tool").is_some());
    assert!(server.find_tool("nonexistent_tool").is_none());
}

#[test]
fn test_server_empty() {
    let server = SdForgeMcpServer::empty();
    assert_eq!(server.tool_count(), 0);
}

#[test]
fn test_server_with_tools() {
    let tool = create_test_tool();
    let metadata = create_test_metadata();
    let instance = McpToolInstance::new(tool, metadata);
    let server = SdForgeMcpServer::with_tools(vec![instance]);
    assert_eq!(server.tool_count(), 1);
    assert!(server.find_tool("test").is_some());
}

#[test]
fn test_build_tool_model() {
    let server = build();
    let instance = server
        .find_tool("coverage_test_tool")
        .expect("coverage tool should be registered");
    let tool_model = server.build_tool_model(instance);
    assert_eq!(tool_model.name.as_ref(), "coverage_test_tool");
    assert!(tool_model.description.is_some());
}

#[tokio::test]
async fn test_server_handler_list_tools() {
    // Use get_all_tools() instead of list_tools() to avoid constructing
    // a RequestContext (which requires a live Peer connection).
    let server = build();
    let tools = server.get_all_tools();
    assert!(!tools.is_empty());
}

#[tokio::test]
async fn test_server_handler_call_tool_success() {
    // Use call_tool_internal() to avoid RequestContext construction.
    let server = build();
    let result = server.call_tool_internal("coverage_test_tool", None);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_server_handler_call_tool_not_found() {
    let server = build();
    let result = server.call_tool_internal("nonexistent", None);
    assert!(result.is_err());
}

#[test]
fn test_server_default() {
    let server = SdForgeMcpServer::default();
    assert!(server.tool_count() > 0);
}

#[test]
fn test_server_clone() {
    let server = build();
    let cloned = server.clone();
    assert_eq!(server.tool_count(), cloned.tool_count());
}

#[test]
fn test_server_with_server_info() {
    let server = SdForgeMcpServer::with_server_info("custom".to_string(), "9.9.9".to_string());
    // If tools exist, name comes from first tool; otherwise from args
    if server.tool_count() == 0 {
        assert_eq!(server.server_name, "custom");
    }
}

#[test]
fn test_server_get_info() {
    let server = build();
    let info = server.get_info();
    assert!(!info.server_info.name.is_empty());
    assert!(!info.server_info.version.is_empty());
}

// ============================================================================
// vuln-0002: end-to-end tests for input_schema validation in call_tool_internal
// ============================================================================
//
// These tests verify that `call_tool_internal` rejects arguments that violate
// the tool's `input_schema` BEFORE the tool's `call()` method is invoked.
// This closes the gap where hand-written `SdForgeTool` implementations that
// forgot to validate arguments in `call()` would accept arbitrary input.
//
// The test tools below intentionally do NOT validate input in `call()` — they
// rely entirely on the entry-point schema validation. If the entry-point
// validation is bypassed, the tool will accept any input (simulating a
// vulnerable hand-written tool).

mod vuln_0002_schema_validation_tests {
    use super::*;
    use crate::mcp::SdForgeTool;
    use rmcp::model::{CallToolResult, ContentBlock, ErrorData as McpError};
    use serde_json::{Value, json};
    use std::sync::Arc;

    /// A tool with a strict schema: requires `name`, allows `age`, rejects
    /// everything else. Its `call()` does NOT validate input — it relies on
    /// the entry-point schema validation in `call_tool_internal`.
    fn create_strict_schema_tool() -> Arc<dyn SdForgeTool> {
        struct StrictSchemaTool;
        impl SdForgeTool for StrictSchemaTool {
            fn name(&self) -> &str {
                "strict_schema_tool"
            }
            fn description(&self) -> &str {
                "Tool with strict input_schema for vuln-0002 tests"
            }
            fn input_schema(&self) -> Value {
                json!({
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "integer"}
                    },
                    "additionalProperties": false
                })
            }
            fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
                // Intentionally does NOT validate input — simulates a
                // hand-written tool that forgot to validate. Entry-point
                // validation must catch invalid input before reaching here.
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    "ok".to_string(),
                )]))
            }
        }
        Arc::new(StrictSchemaTool) as Arc<dyn SdForgeTool>
    }

    /// A tool with an empty schema `{}` — accepts any input.
    fn create_empty_schema_tool() -> Arc<dyn SdForgeTool> {
        struct EmptySchemaTool;
        impl SdForgeTool for EmptySchemaTool {
            fn name(&self) -> &str {
                "empty_schema_tool"
            }
            fn description(&self) -> &str {
                "Tool with empty input_schema"
            }
            fn input_schema(&self) -> Value {
                json!({})
            }
            fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    "ok".to_string(),
                )]))
            }
        }
        Arc::new(EmptySchemaTool) as Arc<dyn SdForgeTool>
    }

    /// A tool whose schema accepts either an object or null
    /// (`type: ["object", "null"]`).
    fn create_union_type_schema_tool() -> Arc<dyn SdForgeTool> {
        struct UnionTypeTool;
        impl SdForgeTool for UnionTypeTool {
            fn name(&self) -> &str {
                "union_type_tool"
            }
            fn description(&self) -> &str {
                "Tool accepting object or null"
            }
            fn input_schema(&self) -> Value {
                json!({"type": ["object", "null"]})
            }
            fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    "ok".to_string(),
                )]))
            }
        }
        Arc::new(UnionTypeTool) as Arc<dyn SdForgeTool>
    }

    /// Build a server containing only the given tool instance.
    fn build_server_with_tool(tool: Arc<dyn SdForgeTool>) -> SdForgeMcpServer {
        let metadata = crate::core::ApiMetadata {
            name: "test".to_string(),
            version: "v1".to_string(),
            description: "test".to_string(),
            cache_ttl: None,
            is_streaming: false,
        };
        let instance = McpToolInstance::new(tool, metadata);
        SdForgeMcpServer::with_tools(vec![instance])
    }

    // --- Strict schema tests ---

    #[test]
    fn test_strict_schema_valid_args_pass() {
        let server = build_server_with_tool(create_strict_schema_tool());
        let result = server.call_tool_internal(
            "strict_schema_tool",
            Some(json!({"name": "Alice", "age": 30})),
        );
        assert!(result.is_ok(), "valid args should pass schema validation");
    }

    #[test]
    fn test_strict_schema_missing_required_rejected() {
        let server = build_server_with_tool(create_strict_schema_tool());
        let result = server.call_tool_internal("strict_schema_tool", Some(json!({"age": 30})));
        assert!(result.is_err(), "missing required field should be rejected");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("missing required field: 'name'"),
            "error should mention missing field 'name', got: {}",
            err.message
        );
    }

    #[test]
    fn test_strict_schema_unknown_field_rejected() {
        let server = build_server_with_tool(create_strict_schema_tool());
        let result = server.call_tool_internal(
            "strict_schema_tool",
            Some(json!({"name": "Alice", "extra": "bad"})),
        );
        assert!(result.is_err(), "unknown field should be rejected");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("unknown field: 'extra'"),
            "error should mention unknown field 'extra', got: {}",
            err.message
        );
    }

    #[test]
    fn test_strict_schema_wrong_type_rejected() {
        // Schema requires type: object, but we pass a string.
        let server = build_server_with_tool(create_strict_schema_tool());
        let result = server.call_tool_internal("strict_schema_tool", Some(json!("not an object")));
        assert!(result.is_err(), "wrong type should be rejected");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("type mismatch"),
            "error should mention type mismatch, got: {}",
            err.message
        );
    }

    #[test]
    fn test_strict_schema_null_args_rejected() {
        // Schema requires type: object; null does not match.
        let server = build_server_with_tool(create_strict_schema_tool());
        let result = server.call_tool_internal("strict_schema_tool", Some(serde_json::Value::Null));
        assert!(
            result.is_err(),
            "null args should be rejected by type: object schema"
        );
    }

    #[test]
    fn test_strict_schema_none_args_skips_validation() {
        // None arguments → no validation, tool's call(None) is invoked.
        let server = build_server_with_tool(create_strict_schema_tool());
        let result = server.call_tool_internal("strict_schema_tool", None);
        assert!(result.is_ok(), "None args should skip schema validation");
    }

    #[test]
    fn test_strict_schema_partial_valid_args_pass() {
        // Only required field provided, optional `age` omitted — valid.
        let server = build_server_with_tool(create_strict_schema_tool());
        let result = server.call_tool_internal("strict_schema_tool", Some(json!({"name": "Bob"})));
        assert!(result.is_ok(), "partial but valid args should pass");
    }

    // --- Empty schema tests ---

    #[test]
    fn test_empty_schema_accepts_any_object() {
        let server = build_server_with_tool(create_empty_schema_tool());
        let result = server.call_tool_internal(
            "empty_schema_tool",
            Some(json!({"anything": "goes", "extra": 123})),
        );
        assert!(result.is_ok(), "empty schema should accept any object");
    }

    #[test]
    fn test_empty_schema_accepts_string() {
        let server = build_server_with_tool(create_empty_schema_tool());
        let result = server.call_tool_internal("empty_schema_tool", Some(json!("any string")));
        assert!(result.is_ok(), "empty schema should accept a string");
    }

    #[test]
    fn test_empty_schema_accepts_null() {
        let server = build_server_with_tool(create_empty_schema_tool());
        let result = server.call_tool_internal("empty_schema_tool", Some(serde_json::Value::Null));
        assert!(result.is_ok(), "empty schema should accept null");
    }

    // --- Union type schema tests ---

    #[test]
    fn test_union_type_schema_accepts_object() {
        let server = build_server_with_tool(create_union_type_schema_tool());
        let result = server.call_tool_internal("union_type_tool", Some(json!({"key": "value"})));
        assert!(
            result.is_ok(),
            "union type [object, null] should accept object"
        );
    }

    #[test]
    fn test_union_type_schema_accepts_null() {
        let server = build_server_with_tool(create_union_type_schema_tool());
        let result = server.call_tool_internal("union_type_tool", Some(serde_json::Value::Null));
        assert!(
            result.is_ok(),
            "union type [object, null] should accept null"
        );
    }

    #[test]
    fn test_union_type_schema_rejects_string() {
        let server = build_server_with_tool(create_union_type_schema_tool());
        let result = server.call_tool_internal("union_type_tool", Some(json!("not allowed")));
        assert!(
            result.is_err(),
            "union type [object, null] should reject string"
        );
    }

    // --- Defense-in-depth: tool is never invoked on invalid input ---

    #[test]
    fn test_invalid_input_never_reaches_tool_call() {
        // If schema validation is bypassed, the tool would return "ok".
        // On rejection, the error message comes from schema_validation, not the tool.
        let server = build_server_with_tool(create_strict_schema_tool());
        let result = server.call_tool_internal(
            "strict_schema_tool",
            Some(json!({"name": "Alice", "extra": "bad"})),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        // The error message must be from schema validation, not from the tool.
        assert!(
            err.message.contains("unknown field"),
            "error should come from schema validation, not tool execution, got: {}",
            err.message
        );
    }
}

// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `SdForgeTool` trait implementations and tool execution behavior.

use super::{create_test_metadata, create_test_tool, exercise_all_tool_methods};
use crate::core::ApiMetadata;
use crate::mcp::{McpToolInstance, SdForgeTool};
use rmcp::model::{CallToolResult, Content, ErrorData as McpError};
use serde_json::Value;
use std::sync::Arc;

#[test]
fn test_api_metadata_creation() {
    let metadata = create_test_metadata();
    assert_eq!(metadata.name, "test_tool");
    assert_eq!(metadata.version, "v1");
    assert_eq!(metadata.description, "A test tool");
}

#[test]
fn test_api_metadata_accessors() {
    let metadata = ApiMetadata {
        name: "my_api".to_string(),
        version: "v2".to_string(),
        description: String::new(),
        cache_ttl: Some(300),
        is_streaming: true,
    };
    assert_eq!(metadata.name(), "my_api");
    assert_eq!(metadata.version(), "v2");
}

#[test]
fn test_tool_call_error_handling() {
    struct ErrorTool;
    impl SdForgeTool for ErrorTool {
        fn name(&self) -> &str {
            "error_tool"
        }
        fn description(&self) -> &str {
            "Tool that returns error"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "string"})
        }
        fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
            Err(McpError::invalid_params("Intentional error", None))
        }
    }
    let tool = Arc::new(ErrorTool) as Arc<dyn SdForgeTool>;
    exercise_all_tool_methods(&*tool);
    let result = tool.call(None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        format!("{:?}", err).contains("Intentional error")
            || err.message.contains("Intentional error")
    );
}

#[test]
fn test_tool_call_with_invalid_input() {
    struct ValidationTool;
    impl SdForgeTool for ValidationTool {
        fn name(&self) -> &str {
            "validation_tool"
        }
        fn description(&self) -> &str {
            "Validates input parameters"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {"required": {"type": "string"}},
                "required": ["required"]
            })
        }
        fn call(&self, input: Option<Value>) -> Result<CallToolResult, McpError> {
            match input {
                Some(value) if value.get("required").is_some() => {
                    Ok(CallToolResult::success(vec![]))
                }
                Some(_) => Err(McpError::invalid_params("Missing required field", None)),
                None => Err(McpError::invalid_params("No input provided", None)),
            }
        }
    }
    let tool = Arc::new(ValidationTool) as Arc<dyn SdForgeTool>;
    exercise_all_tool_methods(&*tool);

    let invalid_input = serde_json::json!({"other": "value"});
    assert!(tool.call(Some(invalid_input)).is_err());
    assert!(tool.call(None).is_err());
    let valid_input = serde_json::json!({"required": "value"});
    assert!(tool.call(Some(valid_input)).is_ok());
}

#[test]
fn test_input_schema_object_type() {
    struct SchemaTool;
    impl SdForgeTool for SchemaTool {
        fn name(&self) -> &str {
            "schema_tool"
        }
        fn description(&self) -> &str {
            "Returns complex schema"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "param1": {"type": "string"},
                    "param2": {"type": "number"}
                },
                "required": ["param1"]
            })
        }
        fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
            Ok(CallToolResult {
                content: vec![],
                structured_content: None,
                is_error: None,
                meta: None,
            })
        }
    }
    let tool = Arc::new(SchemaTool) as Arc<dyn SdForgeTool>;
    exercise_all_tool_methods(&*tool);
    let schema = tool.input_schema();
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"].is_object());
    assert!(schema["required"].is_array());
}

#[test]
fn test_input_schema_primitive_types() {
    struct PrimitiveTool;
    impl SdForgeTool for PrimitiveTool {
        fn name(&self) -> &str {
            "primitive_tool"
        }
        fn description(&self) -> &str {
            "Accepts primitive types"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "string"})
        }
        fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
            Ok(CallToolResult {
                content: vec![],
                structured_content: None,
                is_error: None,
                meta: None,
            })
        }
    }
    let tool = Arc::new(PrimitiveTool) as Arc<dyn SdForgeTool>;
    exercise_all_tool_methods(&*tool);
    let schema = tool.input_schema();
    assert_eq!(schema["type"], "string");
}

#[test]
fn test_tool_execution_with_result() {
    struct ResultTool;
    impl SdForgeTool for ResultTool {
        fn name(&self) -> &str {
            "result_tool"
        }
        fn description(&self) -> &str {
            "Returns a result value"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
        fn call(&self, input: Option<Value>) -> Result<CallToolResult, McpError> {
            let result_value = input.unwrap_or(serde_json::json!({}));
            Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&result_value).unwrap_or_default(),
            )]))
        }
    }
    let tool = Arc::new(ResultTool) as Arc<dyn SdForgeTool>;
    exercise_all_tool_methods(&*tool);
    let input = serde_json::json!({"key": "value"});
    let result = tool.call(Some(input));
    assert!(result.is_ok());
    let response = result.unwrap();
    // rmcp's CallToolResult::success() sets is_error to Some(false)
    assert_eq!(response.is_error, Some(false));
    assert!(!response.content.is_empty());
}

#[test]
fn test_tool_execution_with_empty_response() {
    struct EmptyTool;
    impl SdForgeTool for EmptyTool {
        fn name(&self) -> &str {
            "empty_tool"
        }
        fn description(&self) -> &str {
            "Returns empty response"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
        fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
            Ok(CallToolResult {
                content: vec![],
                structured_content: None,
                is_error: None,
                meta: None,
            })
        }
    }
    let tool = Arc::new(EmptyTool) as Arc<dyn SdForgeTool>;
    let result = tool.call(None);
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.content.is_empty());
}

#[test]
fn test_tool_execution_error_response() {
    struct ErrorResponseTool;
    impl SdForgeTool for ErrorResponseTool {
        fn name(&self) -> &str {
            "error_response_tool"
        }
        fn description(&self) -> &str {
            "Returns error response"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
        fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
            Ok(CallToolResult {
                content: vec![Content::text("Error occurred".to_string())],
                structured_content: None,
                is_error: Some(true),
                meta: None,
            })
        }
    }
    let tool = Arc::new(ErrorResponseTool) as Arc<dyn SdForgeTool>;
    let result = tool.call(None);
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.is_error, Some(true));
    assert!(!response.content.is_empty());
}

#[test]
fn test_metadata_with_cache_ttl() {
    let metadata = ApiMetadata {
        name: "cached_api".to_string(),
        version: "v1".to_string(),
        description: String::new(),
        cache_ttl: Some(600),
        is_streaming: false,
    };
    assert_eq!(metadata.cache_ttl(), Some(600));
}

#[test]
fn test_metadata_streaming_flag() {
    let metadata = ApiMetadata {
        name: "stream_api".to_string(),
        version: "v1".to_string(),
        description: String::new(),
        cache_ttl: None,
        is_streaming: true,
    };
    assert!(metadata.is_streaming());
}

#[test]
fn test_mcp_tool_instance_creation() {
    let tool = create_test_tool();
    let metadata = create_test_metadata();
    let instance = McpToolInstance::new(tool, metadata);
    assert_eq!(instance.tool().name(), "test");
    assert_eq!(instance.metadata().name, "test_tool");
}

#[test]
fn test_mcp_tool_instance_accessors() {
    let tool = create_test_tool();
    let metadata = create_test_metadata();
    let instance = McpToolInstance::new(tool, metadata);
    assert_eq!(instance.tool().name(), "test");
    assert_eq!(instance.metadata().name, "test_tool");
    assert_eq!(instance.metadata().version, "v1");
}

#[test]
fn test_tool_with_complex_input_schema() {
    struct ComplexTool;
    impl SdForgeTool for ComplexTool {
        fn name(&self) -> &str {
            "complex_tool"
        }
        fn description(&self) -> &str {
            "Tool with complex schema"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "nested": {
                        "type": "object",
                        "properties": {
                            "inner": {"type": "string"}
                        }
                    },
                    "array": {
                        "type": "array",
                        "items": {"type": "number"}
                    }
                }
            })
        }
        fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
            Ok(CallToolResult::success(vec![]))
        }
    }
    let tool = ComplexTool;
    assert_eq!(tool.name(), "complex_tool");
    let schema = tool.input_schema();
    assert!(schema["properties"]["nested"].is_object());
    assert!(schema["properties"]["array"]["items"]["type"] == "number");
}

#[test]
fn test_tool_with_none_input() {
    struct NoneInputTool;
    impl SdForgeTool for NoneInputTool {
        fn name(&self) -> &str {
            "none_input_tool"
        }
        fn description(&self) -> &str {
            "Tool that accepts no input"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "null"})
        }
        fn call(&self, input: Option<Value>) -> Result<CallToolResult, McpError> {
            assert!(input.is_none());
            Ok(CallToolResult::success(vec![Content::text(
                "no input".to_string(),
            )]))
        }
    }
    let tool = NoneInputTool;
    let result = tool.call(None);
    assert!(result.is_ok());
}

#[test]
fn test_tool_with_json_input() {
    struct JsonTool;
    impl SdForgeTool for JsonTool {
        fn name(&self) -> &str {
            "json_tool"
        }
        fn description(&self) -> &str {
            "Tool that processes JSON"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
        fn call(&self, input: Option<Value>) -> Result<CallToolResult, McpError> {
            let input = input.unwrap_or_default();
            let text = serde_json::to_string(&input).unwrap_or_default();
            Ok(CallToolResult::success(vec![Content::text(text)]))
        }
    }
    let tool = JsonTool;
    let input = serde_json::json!({"key": "value", "num": 42});
    let result = tool.call(Some(input));
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(!response.content.is_empty());
}

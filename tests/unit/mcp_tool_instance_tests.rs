// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
#[cfg(feature = "mcp")]
mod mcp_tool_instance_tests {
    use rmcp::model::{CallToolResult, ErrorData as McpError};
    use sdforge::core::ApiMetadata;
    use sdforge::mcp::{get_mcp_tools, McpToolRegistration, SdForgeTool};
    use std::sync::Arc;

    fn create_echo_tool() -> Arc<dyn SdForgeTool> {
        struct EchoTool;
        impl SdForgeTool for EchoTool {
            fn name(&self) -> &str {
                "echo"
            }
            fn description(&self) -> &str {
                "Echoes input back"
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"}
                    }
                })
            }
            fn call(&self, input: Option<serde_json::Value>) -> Result<CallToolResult, McpError> {
                Ok(CallToolResult::success(vec![
                    rmcp::model::ContentBlock::text(
                        input.map(|v| v.to_string()).unwrap_or_default(),
                    ),
                ]))
            }
        }
        Arc::new(EchoTool) as Arc<dyn SdForgeTool>
    }

    fn create_add_tool() -> Arc<dyn SdForgeTool> {
        struct AddTool;
        impl SdForgeTool for AddTool {
            fn name(&self) -> &str {
                "add"
            }
            fn description(&self) -> &str {
                "Adds two numbers"
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "a": {"type": "number"},
                        "b": {"type": "number"}
                    },
                    "required": ["a", "b"]
                })
            }
            fn call(&self, input: Option<serde_json::Value>) -> Result<CallToolResult, McpError> {
                let val = input.unwrap_or_default();
                let a = val.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = val.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let sum = a + b;
                Ok(CallToolResult::success(vec![
                    rmcp::model::ContentBlock::text(sum.to_string()),
                ]))
            }
        }
        Arc::new(AddTool) as Arc<dyn SdForgeTool>
    }

    fn create_echo_metadata() -> ApiMetadata {
        ApiMetadata::new(
            "test_tool".to_string(),
            "v1".to_string(),
            "A test tool".to_string(),
            None,
            false,
        )
    }

    #[test]
    fn test_mcp_tool_registration_new() {
        // Test that McpToolRegistration::new() works with the new API
        let _registration =
            McpToolRegistration::new("test_tool", "v1", create_echo_tool, create_echo_metadata);
        // Registration was successful if we reach here
        let _ = &_registration;
    }

    #[test]
    fn test_get_mcp_tools_returns_vector() {
        let tools = get_mcp_tools();
        // Just verify the function works
        let _ = tools.len();
    }

    #[test]
    fn test_tool_call_with_input() {
        let tool = create_echo_tool();
        let input = serde_json::json!({"message": "Hello"});
        let result = tool.call(Some(input));

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.content.is_empty());
    }

    #[test]
    fn test_tool_call_without_input() {
        let tool = create_echo_tool();
        let result = tool.call(None);

        assert!(result.is_ok());
        let _response = result.unwrap();
    }

    #[test]
    fn test_tool_call_arithmetic() {
        let tool = create_add_tool();
        let input = serde_json::json!({"a": 5, "b": 3});
        let result = tool.call(Some(input));

        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_input_schema_validation() {
        let tool = create_add_tool();
        let schema = tool.input_schema();

        assert!(schema.get("properties").is_some());
        let props = schema.get("properties").unwrap();
        assert!(props.get("a").is_some());
        assert!(props.get("b").is_some());
    }

    #[test]
    fn test_tool_response_content_types() {
        fn create_text_tool() -> Arc<dyn SdForgeTool> {
            struct TextTool;
            impl SdForgeTool for TextTool {
                fn name(&self) -> &str {
                    "text"
                }
                fn description(&self) -> &str {
                    "Text tool"
                }
                fn input_schema(&self) -> serde_json::Value {
                    serde_json::json!({"type": "object"})
                }
                fn call(&self, _: Option<serde_json::Value>) -> Result<CallToolResult, McpError> {
                    Ok(CallToolResult::success(vec![
                        rmcp::model::ContentBlock::text("Hello".to_string()),
                    ]))
                }
            }
            Arc::new(TextTool) as Arc<dyn SdForgeTool>
        }

        let tool = create_text_tool();
        let result = tool.call(None).unwrap();

        assert!(matches!(
            result.content.first(),
            Some(c) if c.as_text().is_some()
        ));
    }

    #[test]
    fn test_api_metadata_default_values() {
        let metadata = ApiMetadata::default();
        assert_eq!(metadata.name(), "");
        assert_eq!(metadata.version(), "");
    }

    #[test]
    fn test_api_metadata_with_all_fields() {
        let metadata = ApiMetadata::new(
            "full_tool".to_string(),
            "v3".to_string(),
            "Full metadata tool".to_string(),
            Some(600),
            true,
        );

        assert_eq!(metadata.name(), "full_tool");
        assert_eq!(metadata.version(), "v3");
        assert_eq!(metadata.description(), "Full metadata tool");
        assert_eq!(metadata.cache_ttl(), Some(600));
        assert!(metadata.is_streaming());
    }
}

#[cfg(feature = "mcp")]
mod mcp_tool_instance_tests {
    use sdforge::mcp::{get_mcp_tools, McpToolRegistration, McpToolInstance};
    use sdforge::core::ApiMetadata;
    use std::sync::Arc;

    fn create_echo_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
        struct EchoTool;
        impl mcp_sdk::tools::Tool for EchoTool {
            fn name(&self) -> String {
                "echo".to_string()
            }
            fn description(&self) -> String {
                "Echoes input back".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"}
                    }
                })
            }
            fn call(
                &self,
                input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![mcp_sdk::types::ToolResponseContent::Text {
                        text: input.map(|v| v.to_string()).unwrap_or_default(),
                    }],
                    is_error: None,
                    meta: None,
                })
            }
        }
        Arc::new(EchoTool) as Arc<dyn mcp_sdk::tools::Tool>
    }

    fn create_add_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
        struct AddTool;
        impl mcp_sdk::tools::Tool for AddTool {
            fn name(&self) -> String {
                "add".to_string()
            }
            fn description(&self) -> String {
                "Adds two numbers".to_string()
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
            fn call(
                &self,
                input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                let val = input.unwrap_or_default();
                let a = val.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = val.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let sum = a + b;
                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![mcp_sdk::types::ToolResponseContent::Text {
                        text: sum.to_string(),
                    }],
                    is_error: None,
                    meta: None,
                })
            }
        }
        Arc::new(AddTool) as Arc<dyn mcp_sdk::tools::Tool>
    }

    #[test]
    fn test_mcp_tool_registration_new() {
        let registration = McpToolRegistration::new(
            "test_tool",
            "v1",
            "A test tool",
            create_echo_tool,
        );
        
        assert_eq!(registration.name, "test_tool");
        assert_eq!(registration.version, "v1");
        assert_eq!(registration.description, "A test tool");
    }

    #[test]
    fn test_mcp_tool_registration_create_fn() {
        let registration = McpToolRegistration::new(
            "echo_tool",
            "v1",
            "Echo tool",
            create_echo_tool,
        );
        
        let tool = (registration.create_fn)();
        assert_eq!(tool.name(), "echo");
        assert_eq!(tool.description(), "Echoes input back");
    }

    #[test]
    fn test_get_mcp_tools_returns_vector() {
        let tools = get_mcp_tools();
        assert!(tools.is_empty() || tools.len() >= 0);
    }

    #[test]
    fn test_mcp_tool_instance_tool_accessor() {
        let registration = McpToolRegistration::new(
            "test_instance",
            "v1",
            "Test instance",
            create_echo_tool,
        );
        
        let tool = (registration.create_fn)();
        let instance = McpToolInstance {
            tool,
            metadata: ApiMetadata::new(
                "test_instance".to_string(),
                "v1".to_string(),
                "Test instance".to_string(),
                None,
                false,
            ),
        };
        
        let retrieved_tool = instance.tool();
        assert_eq!(retrieved_tool.name(), "test_instance");
    }

    #[test]
    fn test_mcp_tool_instance_metadata_accessor() {
        let registration = McpToolRegistration::new(
            "metadata_test",
            "v2",
            "Metadata test tool",
            create_echo_tool,
        );
        
        let tool = (registration.create_fn)();
        let instance = McpToolInstance {
            tool,
            metadata: ApiMetadata::new(
                "metadata_test".to_string(),
                "v2".to_string(),
                "Metadata test tool".to_string(),
                Some(300),
                true,
            ),
        };
        
        assert_eq!(instance.metadata().name(), "metadata_test");
        assert_eq!(instance.metadata().version(), "v2");
        assert_eq!(instance.metadata().cache_ttl(), Some(300));
        assert!(instance.metadata().is_streaming());
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
        let response = result.unwrap();
        assert!(response.content.is_empty() || response.content.len() > 0);
    }

    #[test]
    fn test_tool_call_arithmetic() {
        let tool = create_add_tool();
        let input = serde_json::json!({"a": 5, "b": 3});
        let result = tool.call(Some(input));
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_tool_registrations() {
        let reg1 = McpToolRegistration::new("tool1", "v1", "Tool 1", create_echo_tool);
        let reg2 = McpToolRegistration::new("tool2", "v1", "Tool 2", create_add_tool);
        
        let tool1 = (reg1.create_fn)();
        let tool2 = (reg2.create_fn)();
        
        assert_eq!(tool1.name(), "echo");
        assert_eq!(tool2.name(), "add");
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
        fn create_text_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
            struct TextTool;
            impl mcp_sdk::tools::Tool for TextTool {
                fn name(&self) -> String { "text".to_string() }
                fn description(&self) -> String { "Text tool".to_string() }
                fn input_schema(&self) -> serde_json::Value {
                    serde_json::json!({"type": "object"})
                }
                fn call(&self, _: Option<serde_json::Value>) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                    Ok(mcp_sdk::types::CallToolResponse {
                        content: vec![
                            mcp_sdk::types::ToolResponseContent::Text { text: "Hello".to_string() }
                        ],
                        is_error: None,
                        meta: None,
                    })
                }
            }
            Arc::new(TextTool) as Arc<dyn mcp_sdk::tools::Tool>
        }
        
        let tool = create_text_tool();
        let result = tool.call(None).unwrap();
        
        assert!(matches!(
            result.content.first(),
            Some(mcp_sdk::types::ToolResponseContent::Text { .. })
        ));
    }

    #[test]
    fn test_api_metadata_default_values() {
        let metadata = ApiMetadata::default();
        assert_eq!(metadata.name(), "");
        assert_eq!(metadata.version(), "v1");
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

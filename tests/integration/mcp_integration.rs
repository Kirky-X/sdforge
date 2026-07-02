// MCP Integration Tests
// Covers TC-INT-002, TC-INT-003

#[cfg(feature = "mcp")]
mod mcp_tests {
    use sdforge::mcp::build;

    #[tokio::test]
    async fn test_mcp_server_builds() {
        // Just verify build() can be called without panicking
        let _server = build();
    }
}

#[cfg(feature = "mcp")]
mod mcp_registration_tests {
    use rmcp::model::{CallToolResult, ErrorData as McpError};
    use sdforge::core::ApiMetadata;
    use sdforge::mcp::{McpToolRegistration, SdForgeTool};
    use std::sync::Arc;

    struct TestTool;
    impl SdForgeTool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }
        fn description(&self) -> &str {
            "A test tool"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            })
        }
        fn call(
            &self,
            _input: Option<serde_json::Value>,
        ) -> Result<CallToolResult, McpError> {
            Ok(CallToolResult {
                content: vec![],
                structured_content: None,
                is_error: None,
                meta: None,
            })
        }
    }

    #[test]
    fn test_mcp_tool_registration() {
        // Test that McpToolRegistration::new() works with the new API
        fn create_tool() -> Arc<dyn SdForgeTool> {
            Arc::new(TestTool)
        }
        fn create_metadata() -> ApiMetadata {
            ApiMetadata::new(
                "test_tool".to_string(),
                "v1".to_string(),
                "A test tool".to_string(),
                None,
                false,
            )
        }
        let _registration = McpToolRegistration::new("test_tool", "v1", create_tool, create_metadata);

        // Verify the registration was created successfully (construction without panic)
        let _ = &_registration;
    }

    #[test]
    fn test_mcp_tool_creation() {
        // Create a tool instance directly to verify the SdForgeTool implementation works
        let tool = TestTool;
        assert_eq!(tool.name(), "test_tool");
    }
}

#[cfg(all(feature = "http", feature = "mcp"))]
mod dual_protocol_tests {
    use sdforge::http::build as http_build;
    use sdforge::mcp::build as mcp_build;

    #[tokio::test]
    async fn test_both_protocols_build() {
        // Just verify both builds can be called without panicking
        let _http_app = http_build();
        let _mcp_server = mcp_build();
    }
}

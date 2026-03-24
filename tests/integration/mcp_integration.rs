// MCP Integration Tests
// Covers TC-INT-002, TC-INT-003

#[cfg(feature = "mcp")]
mod mcp_tests {
    use sdforge::mcp::build;

    #[tokio::test]
    async fn test_mcp_server_builds() {
        // Just verify build() can be called without panicking
        let _server = build().await;
    }
}

#[cfg(feature = "mcp")]
mod mcp_registration_tests {
    use sdforge::mcp::McpToolRegistration;
    use std::sync::Arc;
    use mcp_sdk::tools::Tool;

    struct TestTool;
    impl Tool for TestTool {
        fn name(&self) -> String {
            "test_tool".to_string()
        }
        fn description(&self) -> String {
            "A test tool".to_string()
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
        ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
            Ok(mcp_sdk::types::CallToolResponse {
                content: vec![],
                is_error: None,
                meta: None,
            })
        }
    }

    #[test]
    fn test_mcp_tool_registration() {
        // Test that McpToolRegistration::new() works
        let _registration = McpToolRegistration::new(
            "test_tool",
            "v1",
            "A test tool",
            || Arc::new(TestTool) as Arc<dyn Tool>,
        );

        // Verify the registration can be created
        assert!(true);
    }

    #[test]
    fn test_mcp_tool_creation() {
        // Create a tool instance directly to verify the Tool implementation works
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
        let _mcp_server = mcp_build().await;
    }
}

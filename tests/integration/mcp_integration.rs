// MCP Integration Tests
// Covers TC-INT-002, TC-INT-003

#[cfg(feature = "mcp")]
mod mcp_tests {
    use sdforge::mcp::build;

    #[tokio::test]
    async fn test_mcp_server_builds() {
        let server = build().await;
        assert!(server.is_ok());
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
            input: Option<serde_json::Value>,
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
        let registration = McpToolRegistration {
            name: "test_tool",
            version: "v1",
            description: "A test tool",
            create_fn: || Arc::new(TestTool) as Arc<dyn Tool>,
        };

        assert_eq!(registration.name, "test_tool");
        assert_eq!(registration.version, "v1");
    }

    #[test]
    fn test_mcp_tool_creation() {
        let registration = McpToolRegistration {
            name: "test_tool",
            version: "v1",
            description: "A test tool",
            create_fn: || Arc::new(TestTool) as Arc<dyn Tool>,
        };

        let tool = (registration.create_fn)();
        assert_eq!(tool.name(), "test_tool");
    }
}

#[cfg(all(feature = "http", feature = "mcp"))]
mod dual_protocol_tests {
    use sdforge::http::build as http_build;
    use sdforge::mcp::build as mcp_build;

    #[tokio::test]
    async fn test_both_protocols_build() {
        let http_app = http_build();
        assert!(http_app.is_ok());

        let mcp_server = mcp_build().await;
        assert!(mcp_server.is_ok());
    }
}

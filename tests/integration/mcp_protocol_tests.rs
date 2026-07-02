// MCP Protocol Integration Tests
// Comprehensive integration tests for MCP protocol functionality
// Tests cover: tool discovery, request handling, response formatting,
// input validation, and error handling
//
// Migrated from mcp_sdk to rmcp: JSON-RPC transport tests removed (handled
// internally by rmcp). Tests now focus on SdForgeTool behavior and
// SdForgeMcpServer (ServerHandler) integration.

#[cfg(feature = "mcp")]
mod mcp_protocol_tests {
    use rmcp::handler::server::ServerHandler;
    use rmcp::model::{CallToolResult, Content, ErrorData as McpError};
    use sdforge::core::{ApiMetadata, Registration};
    use sdforge::mcp::{
        get_mcp_tools, McpToolInstance, McpToolRegistration, SdForgeMcpServer, SdForgeTool,
    };
    use serde_json::Value;
    use std::sync::Arc;

    // ============================================================================
    // Helper Functions
    // ============================================================================

    /// Creates a simple echo tool for testing
    fn create_echo_tool() -> Arc<dyn SdForgeTool> {
        struct EchoTool;
        impl SdForgeTool for EchoTool {
            fn name(&self) -> &str {
                "echo"
            }
            fn description(&self) -> &str {
                "Echoes the input message back"
            }
            fn input_schema(&self) -> Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"},
                        "count": {"type": "integer", "default": 1}
                    }
                })
            }
            fn call(&self, input: Option<Value>) -> Result<CallToolResult, McpError> {
                let message = input
                    .as_ref()
                    .and_then(|v| v.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("");

                let count = input
                    .as_ref()
                    .and_then(|v| v.get("count"))
                    .and_then(|c| c.as_i64())
                    .unwrap_or(1) as usize;

                let output = format!("{}x{}", message, count);
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
        }
        Arc::new(EchoTool) as Arc<dyn SdForgeTool>
    }

    /// Creates a math tool for testing with multiple operations
    fn create_math_tool() -> Arc<dyn SdForgeTool> {
        struct MathTool;
        impl SdForgeTool for MathTool {
            fn name(&self) -> &str {
                "math"
            }
            fn description(&self) -> &str {
                "Performs basic arithmetic operations"
            }
            fn input_schema(&self) -> Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "a": {"type": "number"},
                        "b": {"type": "number"},
                        "operation": {
                            "type": "string",
                            "enum": ["add", "subtract", "multiply", "divide"]
                        }
                    },
                    "required": ["a", "b", "operation"]
                })
            }
            fn call(&self, input: Option<Value>) -> Result<CallToolResult, McpError> {
                let input =
                    input.ok_or_else(|| McpError::invalid_params("Input required", None))?;
                let a = input
                    .get("a")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::invalid_params("Missing 'a'", None))?;
                let b = input
                    .get("b")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::invalid_params("Missing 'b'", None))?;
                let op = input
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::invalid_params("Missing 'operation'", None))?;

                let result = match op {
                    "add" => a + b,
                    "subtract" => a - b,
                    "multiply" => a * b,
                    "divide" => {
                        if b == 0.0 {
                            return Err(McpError::invalid_params("Division by zero", None));
                        }
                        a / b
                    }
                    _ => {
                        return Err(McpError::invalid_params(
                            format!("Unknown operation: {}", op),
                            None,
                        ))
                    }
                };

                Ok(CallToolResult::success(vec![Content::text(
                    result.to_string(),
                )]))
            }
        }
        Arc::new(MathTool) as Arc<dyn SdForgeTool>
    }

    /// Creates a tool that always returns an error
    fn create_error_tool() -> Arc<dyn SdForgeTool> {
        struct ErrorTool;
        impl SdForgeTool for ErrorTool {
            fn name(&self) -> &str {
                "error_tool"
            }
            fn description(&self) -> &str {
                "Always returns an error"
            }
            fn input_schema(&self) -> Value {
                serde_json::json!({"type": "object"})
            }
            fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
                Err(McpError::invalid_params("Intentional test error", None))
            }
        }
        Arc::new(ErrorTool) as Arc<dyn SdForgeTool>
    }

    /// Creates a tool with no parameters
    fn create_no_param_tool() -> Arc<dyn SdForgeTool> {
        struct NoParamTool;
        impl SdForgeTool for NoParamTool {
            fn name(&self) -> &str {
                "no_param"
            }
            fn description(&self) -> &str {
                "Tool with no parameters"
            }
            fn input_schema(&self) -> Value {
                serde_json::json!({"type": "object"})
            }
            fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
                Ok(CallToolResult::success(vec![Content::text(
                    "no params needed".to_string(),
                )]))
            }
        }
        Arc::new(NoParamTool) as Arc<dyn SdForgeTool>
    }

    /// Creates a tool with complex nested schema
    fn create_complex_tool() -> Arc<dyn SdForgeTool> {
        struct ComplexTool;
        impl SdForgeTool for ComplexTool {
            fn name(&self) -> &str {
                "complex"
            }
            fn description(&self) -> &str {
                "Tool with complex nested schema"
            }
            fn input_schema(&self) -> Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "user": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "age": {"type": "integer"}
                            },
                            "required": ["name"]
                        }
                    },
                    "required": ["user"]
                })
            }
            fn call(&self, input: Option<Value>) -> Result<CallToolResult, McpError> {
                let input = input.unwrap_or_default();
                let name = input
                    .get("user")
                    .and_then(|u| u.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown");
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Hello, {}!",
                    name
                ))]))
            }
        }
        Arc::new(ComplexTool) as Arc<dyn SdForgeTool>
    }

    // ============================================================================
    // SdForgeMcpServer Tests (ServerHandler integration)
    // ============================================================================

    /// Test: Create server with explicit tools
    #[test]
    fn test_server_creation_with_tools() {
        let tools = vec![
            McpToolInstance::new(create_echo_tool(), ApiMetadata::default()),
            McpToolInstance::new(create_math_tool(), ApiMetadata::default()),
        ];
        let server = SdForgeMcpServer::with_tools(tools);
        assert_eq!(server.tool_count(), 2);
    }

    /// Test: Create empty server
    #[test]
    fn test_server_creation_empty() {
        let server = SdForgeMcpServer::empty();
        assert_eq!(server.tool_count(), 0);
    }

    /// Test: Find tool by name
    #[test]
    fn test_find_tool_by_name() {
        let tools = vec![
            McpToolInstance::new(create_echo_tool(), ApiMetadata::default()),
            McpToolInstance::new(create_math_tool(), ApiMetadata::default()),
        ];
        let server = SdForgeMcpServer::with_tools(tools);

        assert!(server.find_tool("echo").is_some());
        assert!(server.find_tool("math").is_some());
        assert!(server.find_tool("nonexistent").is_none());
    }

    /// Test: Server get_info returns server info
    #[test]
    fn test_server_get_info() {
        let server = SdForgeMcpServer::empty();
        let info = server.get_info();
        assert!(!info.server_info.name.is_empty());
        assert!(!info.server_info.version.is_empty());
    }

    // ============================================================================
    // Tool Execution Tests
    // ============================================================================

    /// Test: Echo tool with valid input
    #[test]
    fn test_echo_tool_execution() {
        let tool = create_echo_tool();
        let input = serde_json::json!({"message": "hello", "count": 3});

        let result = tool.call(Some(input));
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.content.is_empty());

        // Verify content is text
        assert!(matches!(
            response.content.first(),
            Some(c) if c.as_text().is_some()
        ));
    }

    /// Test: Echo tool with default count
    #[test]
    fn test_echo_tool_default_count() {
        let tool = create_echo_tool();
        let input = serde_json::json!({"message": "test"});

        let result = tool.call(Some(input)).unwrap();
        assert!(!result.content.is_empty());
    }

    /// Test: Math tool addition
    #[test]
    fn test_math_tool_addition() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 5, "b": 3, "operation": "add"});

        let result = tool.call(Some(input));
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.content.is_empty());
    }

    /// Test: Math tool subtraction
    #[test]
    fn test_math_tool_subtraction() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 10, "b": 4, "operation": "subtract"});

        let result = tool.call(Some(input)).unwrap();
        assert!(!result.content.is_empty());
    }

    /// Test: Math tool multiplication
    #[test]
    fn test_math_tool_multiplication() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 6, "b": 7, "operation": "multiply"});

        let result = tool.call(Some(input)).unwrap();
        assert!(!result.content.is_empty());
    }

    /// Test: Math tool division
    #[test]
    fn test_math_tool_division() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 20, "b": 4, "operation": "divide"});

        let result = tool.call(Some(input));
        assert!(result.is_ok());
    }

    /// Test: Math tool division by zero
    #[test]
    fn test_math_tool_division_by_zero() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 10, "b": 0, "operation": "divide"});

        let result = tool.call(Some(input));
        assert!(result.is_err());
    }

    /// Test: Math tool missing required field
    #[test]
    fn test_math_tool_missing_field() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 5}); // Missing 'b' and 'operation'

        let result = tool.call(Some(input));
        assert!(result.is_err());
    }

    /// Test: Math tool invalid operation
    #[test]
    fn test_math_tool_invalid_operation() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 5, "b": 3, "operation": "modulo"});

        let result = tool.call(Some(input));
        assert!(result.is_err());
    }

    /// Test: Math tool no input
    #[test]
    fn test_math_tool_no_input() {
        let tool = create_math_tool();
        let result = tool.call(None);
        assert!(result.is_err());
    }

    /// Test: Error tool returns error
    #[test]
    fn test_error_tool_returns_error() {
        let tool = create_error_tool();
        let result = tool.call(None);
        assert!(result.is_err());
    }

    /// Test: No-param tool works without input
    #[test]
    fn test_no_param_tool() {
        let tool = create_no_param_tool();
        let result = tool.call(None);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.content.is_empty());
    }

    /// Test: Complex tool with valid input
    #[test]
    fn test_complex_tool_valid_input() {
        let tool = create_complex_tool();
        let input = serde_json::json!({"user": {"name": "Alice", "age": 30}});

        let result = tool.call(Some(input));
        assert!(result.is_ok());
    }

    /// Test: Complex tool with missing required field
    #[test]
    fn test_complex_tool_missing_field() {
        let tool = create_complex_tool();
        let input = serde_json::json!({"user": {"age": 30}}); // Missing 'name'

        // Complex tool doesn't validate required fields in call, only in schema
        let result = tool.call(Some(input));
        assert!(result.is_ok()); // Uses default "unknown"
    }

    // ============================================================================
    // Input Schema Validation Tests
    // ============================================================================

    /// Test: Echo tool schema structure
    #[test]
    fn test_echo_tool_schema() {
        let tool = create_echo_tool();
        let schema = tool.input_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["message"].is_object());
        assert!(schema["properties"]["count"].is_object());
    }

    /// Test: Math tool schema with required fields
    #[test]
    fn test_math_tool_schema() {
        let tool = create_math_tool();
        let schema = tool.input_schema();

        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("a")));
        assert!(required.contains(&serde_json::json!("b")));
        assert!(required.contains(&serde_json::json!("operation")));

        // Verify enum constraint
        let op_schema = &schema["properties"]["operation"];
        let enum_values = op_schema["enum"].as_array().unwrap();
        assert!(enum_values.contains(&serde_json::json!("add")));
        assert!(enum_values.contains(&serde_json::json!("subtract")));
        assert!(enum_values.contains(&serde_json::json!("multiply")));
        assert!(enum_values.contains(&serde_json::json!("divide")));
    }

    /// Test: Complex tool nested schema
    #[test]
    fn test_complex_tool_nested_schema() {
        let tool = create_complex_tool();
        let schema = tool.input_schema();

        let user_props = &schema["properties"]["user"]["properties"];
        assert!(user_props.get("name").is_some());
        assert!(user_props.get("age").is_some());

        let user_required = schema["properties"]["user"]["required"].as_array().unwrap();
        assert!(user_required.contains(&serde_json::json!("name")));
    }

    /// Test: Schema is valid JSON
    #[test]
    fn test_schema_serialization() {
        let tool = create_complex_tool();
        let schema = tool.input_schema();

        let serialized = serde_json::to_string(&schema).unwrap();
        let deserialized: Value = serde_json::from_str(&serialized).unwrap();
        assert!(deserialized.is_object());
    }

    // ============================================================================
    // Response Content Tests
    // ============================================================================

    /// Test: Text content type
    #[test]
    fn test_text_content_type() {
        let tool = create_echo_tool();
        let input = serde_json::json!({"message": "test"});

        let response = tool.call(Some(input)).unwrap();
        assert!(!response.content.is_empty());

        // Verify content is text type
        assert!(matches!(
            response.content.first(),
            Some(c) if c.as_text().is_some()
        ));
    }

    /// Test: is_error flag is None for successful calls
    #[test]
    fn test_is_error_none_on_success() {
        let tool = create_echo_tool();
        let input = serde_json::json!({"message": "test"});

        let response = tool.call(Some(input)).unwrap();
        assert!(response.is_error.is_none() || response.is_error == Some(false));
    }

    /// Test: meta field is None by default
    #[test]
    fn test_meta_none_by_default() {
        let tool = create_no_param_tool();
        let response = tool.call(None).unwrap();
        assert!(response.meta.is_none());
    }

    /// Test: structured_content is None by default
    #[test]
    fn test_structured_content_none_by_default() {
        let tool = create_no_param_tool();
        let response = tool.call(None).unwrap();
        assert!(response.structured_content.is_none());
    }

    // ============================================================================
    // Multiple Tools Coexistence Tests
    // ============================================================================

    /// Test: Multiple tools can coexist with different names
    #[test]
    fn test_multiple_tools_coexistence() {
        let tools = vec![
            McpToolInstance::new(create_echo_tool(), ApiMetadata::default()),
            McpToolInstance::new(create_math_tool(), ApiMetadata::default()),
            McpToolInstance::new(create_error_tool(), ApiMetadata::default()),
            McpToolInstance::new(create_no_param_tool(), ApiMetadata::default()),
            McpToolInstance::new(create_complex_tool(), ApiMetadata::default()),
        ];
        let server = SdForgeMcpServer::with_tools(tools);

        assert_eq!(server.tool_count(), 5);
        assert!(server.find_tool("echo").is_some());
        assert!(server.find_tool("math").is_some());
        assert!(server.find_tool("error_tool").is_some());
        assert!(server.find_tool("no_param").is_some());
        assert!(server.find_tool("complex").is_some());
    }

    // ============================================================================
    // get_mcp_tools Integration Tests
    // ============================================================================

    /// Test: get_mcp_tools returns instances with accessible metadata
    #[test]
    fn test_get_mcp_tools_metadata_access() {
        let tools = get_mcp_tools();
        for instance in tools {
            let metadata = instance.metadata();
            let _ = metadata.name();
            let _ = metadata.version();
            let _ = metadata.cache_ttl();
            let _ = metadata.is_streaming();
        }
    }

    /// Test: get_mcp_tools returns instances with accessible tools
    #[test]
    fn test_get_mcp_tools_tool_access() {
        let tools = get_mcp_tools();
        for instance in tools {
            let tool = instance.tool();
            let _ = tool.name();
            let _ = tool.description();
            let _ = tool.input_schema();
        }
    }

    // ============================================================================
    // McpToolRegistration Tests
    // ============================================================================

    /// Test: Registration creates valid tool instances
    #[test]
    fn test_registration_creates_valid_tool() {
        fn create_test_tool() -> Arc<dyn SdForgeTool> {
            create_echo_tool()
        }
        fn create_test_metadata() -> ApiMetadata {
            ApiMetadata::default()
        }
        let reg = McpToolRegistration::new("echo", "v1", create_test_tool, create_test_metadata);
        assert_eq!(reg.name, "echo");
        assert_eq!(reg.version, "v1");

        let tool = reg.create();
        assert_eq!(tool.name(), "echo");
    }

    /// Test: Registration metadata is accessible
    #[test]
    fn test_registration_metadata_access() {
        fn create_test_tool() -> Arc<dyn SdForgeTool> {
            create_math_tool()
        }
        fn create_test_metadata() -> ApiMetadata {
            ApiMetadata::new(
                "math".to_string(),
                "v2".to_string(),
                "Math tool".to_string(),
                Some(300),
                false,
            )
        }
        let reg = McpToolRegistration::new("math", "v2", create_test_tool, create_test_metadata);

        let metadata = reg.metadata();
        assert_eq!(metadata.name(), "math");
        assert_eq!(metadata.version(), "v2");
    }
}

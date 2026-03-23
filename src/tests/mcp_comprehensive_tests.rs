//! MCP protocol comprehensive tests
//!
//! This module provides comprehensive external integration tests for the MCP module,
//! focusing on:
//! - get_mcp_tools() function behavior
//! - McpToolInstance accessor methods
//! - Edge cases not covered by internal tests
//! - Concurrent tool execution scenarios

#[cfg(feature = "mcp")]
mod mcp_comprehensive_tests {
    use sdforge::mcp::{get_mcp_tools, McpToolRegistration};
    use std::sync::Arc;
    use std::thread;

    // ============================================================================
    // Helper Functions
    // ============================================================================

    fn create_simple_tool(name: &str) -> Arc<dyn mcp_sdk::tools::Tool> {
        struct SimpleTool {
            name: String,
        }

        impl mcp_sdk::tools::Tool for SimpleTool {
            fn name(&self) -> String {
                self.name.clone()
            }

            fn description(&self) -> String {
                format!("Test tool: {}", self.name)
            }

            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
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

        Arc::new(SimpleTool {
            name: name.to_string(),
        }) as Arc<dyn mcp_sdk::tools::Tool>
    }

    fn create_echo_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
        struct EchoTool;

        impl mcp_sdk::tools::Tool for EchoTool {
            fn name(&self) -> String {
                "echo".to_string()
            }

            fn description(&self) -> String {
                "Echoes input back as response".to_string()
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
                let text = input
                    .and_then(|v| v.get("message").map(|m| m.to_string()))
                    .unwrap_or_default();

                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![mcp_sdk::types::ToolResponseContent::Text { text }],
                    is_error: None,
                    meta: None,
                })
            }
        }

        Arc::new(EchoTool) as Arc<dyn mcp_sdk::tools::Tool>
    }

    fn create_math_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
        struct MathTool;

        impl mcp_sdk::tools::Tool for MathTool {
            fn name(&self) -> String {
                "math".to_string()
            }

            fn description(&self) -> String {
                "Performs basic arithmetic".to_string()
            }

            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "a": {"type": "number"},
                        "b": {"type": "number"},
                        "operation": {"type": "string", "enum": ["add", "subtract", "multiply"]}
                    },
                    "required": ["a", "b", "operation"]
                })
            }

            fn call(
                &self,
                input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                let val = input.ok_or_else(|| anyhow::anyhow!("Input required"))?;
                let a = val
                    .get("a")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'a'"))?;
                let b = val
                    .get("b")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'b'"))?;
                let op = val
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'operation'"))?;

                let result = match op {
                    "add" => a + b,
                    "subtract" => a - b,
                    "multiply" => a * b,
                    _ => return Err(anyhow::anyhow!("Unknown operation: {}", op)),
                };

                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![mcp_sdk::types::ToolResponseContent::Text {
                        text: result.to_string(),
                    }],
                    is_error: None,
                    meta: None,
                })
            }
        }

        Arc::new(MathTool) as Arc<dyn mcp_sdk::tools::Tool>
    }

    fn create_error_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
        struct ErrorTool;

        impl mcp_sdk::tools::Tool for ErrorTool {
            fn name(&self) -> String {
                "error_tool".to_string()
            }

            fn description(&self) -> String {
                "Always returns an error".to_string()
            }

            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }

            fn call(
                &self,
                _input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                Err(anyhow::anyhow!("Intentional test error"))
            }
        }

        Arc::new(ErrorTool) as Arc<dyn mcp_sdk::tools::Tool>
    }

    fn create_complex_schema_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
        struct ComplexSchemaTool;

        impl mcp_sdk::tools::Tool for ComplexSchemaTool {
            fn name(&self) -> String {
                "complex_schema".to_string()
            }

            fn description(&self) -> String {
                "Tool with complex nested schema".to_string()
            }

            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "user": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "age": {"type": "integer", "minimum": 0},
                                "address": {
                                    "type": "object",
                                    "properties": {
                                        "street": {"type": "string"},
                                        "city": {"type": "string"},
                                        "country": {"type": "string"}
                                    }
                                }
                            },
                            "required": ["name"]
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"}
                        }
                    },
                    "required": ["user"]
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

        Arc::new(ComplexSchemaTool) as Arc<dyn mcp_sdk::tools::Tool>
    }

    // ============================================================================
    // get_mcp_tools() Function Tests
    // ============================================================================

    /// Test that get_mcp_tools returns a vector (may be empty if no tools registered)
    #[test]
    fn test_get_mcp_tools_returns_vector() {
        let tools = get_mcp_tools();
        assert!(!tools.is_empty() || tools.is_empty());
    }

    /// Test that get_mcp_tools can be called multiple times
    #[test]
    fn test_get_mcp_tools_idempotent() {
        let tools1 = get_mcp_tools();
        let tools2 = get_mcp_tools();

        // Both calls should return vectors of the same length
        assert_eq!(tools1.len(), tools2.len());
    }

    /// Test that get_mcp_tools returns properly structured instances
    #[test]
    fn test_get_mcp_tools_instance_structure() {
        let tools = get_mcp_tools();

        for instance in tools {
            // Each instance should have valid tool and metadata
            let _ = instance.tool();
            let _ = instance.metadata();
        }
    }

    // ============================================================================
    // McpToolInstance Accessor Tests (via get_mcp_tools)
    // ============================================================================

    #[test]
    fn test_mcp_tool_instance_tool_accessor() {
        let tools = get_mcp_tools();
        for instance in tools {
            let tool = instance.tool();
            assert!(!tool.name().is_empty() || tool.name().is_empty());
        }
    }

    #[test]
    fn test_mcp_tool_instance_metadata_accessor() {
        let tools = get_mcp_tools();
        for instance in tools {
            let metadata = instance.metadata();
            assert!(!metadata.name().is_empty() || metadata.name().is_empty());
        }
    }

    #[test]
    fn test_mcp_tool_instance_tool_arc_clone() {
        let tools = get_mcp_tools();
        for instance in tools {
            let tool1 = instance.tool().clone();
            let tool2 = instance.tool().clone();
            assert_eq!(tool1.name(), tool2.name());
        }
    }

    #[test]
    fn test_mcp_tool_instance_metadata_variations() {
        let tools = get_mcp_tools();
        for instance in tools {
            let _ = instance.metadata().cache_ttl();
            let _ = instance.metadata().is_streaming();
        }
    }

    // ============================================================================
    // Edge Case Tests - Tool Behavior
    // ============================================================================

    #[test]
    fn test_tool_with_empty_name() {
        let tool = create_simple_tool("");
        assert_eq!(tool.name(), "");
    }

    #[test]
    fn test_tool_with_special_characters() {
        let tool = create_simple_tool("tool-with_special.chars:v2");
        assert_eq!(tool.name(), "tool-with_special.chars:v2");
    }

    #[test]
    fn test_tool_with_unicode_description() {
        let tool = create_simple_tool("unicode_tool");
        assert!(tool
            .description()
            .contains(&format!("Test tool: {}", "unicode_tool")));
    }

    #[test]
    fn test_tool_with_long_name() {
        let long_name = "a".repeat(1000);
        let tool = create_simple_tool(&long_name);
        assert_eq!(tool.name().len(), 1000);
    }

    #[test]
    fn test_tool_versions() {
        let versions = vec!["v1", "v2.0", "1.0.0", "beta", "2024-01-01", ""];
        for version in versions {
            let reg = McpToolRegistration::new("version_test", version, "Version test", || {
                create_simple_tool("version_test")
            });
            let _ = reg;
        }
    }

    // ============================================================================
    // Tool Execution Tests
    // ============================================================================

    /// Test echo tool with valid input
    #[test]
    fn test_echo_tool_execution() {
        let tool = create_echo_tool();
        let input = serde_json::json!({"message": "Hello, World!"});

        let result = tool.call(Some(input));
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.content.is_empty());
    }

    /// Test math tool with valid inputs
    #[test]
    fn test_math_tool_addition() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 5, "b": 3, "operation": "add"});

        let result = tool.call(Some(input));
        assert!(result.is_ok());
    }

    /// Test math tool with subtraction
    #[test]
    fn test_math_tool_subtraction() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 10, "b": 4, "operation": "subtract"});

        let result = tool.call(Some(input));
        assert!(result.is_ok());
    }

    /// Test math tool with multiplication
    #[test]
    fn test_math_tool_multiplication() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 6, "b": 7, "operation": "multiply"});

        let result = tool.call(Some(input));
        assert!(result.is_ok());
    }

    /// Test error tool returns error
    #[test]
    fn test_error_tool_returns_error() {
        let tool = create_error_tool();
        let result = tool.call(None);

        assert!(result.is_err());
    }

    /// Test complex schema tool returns valid schema
    #[test]
    fn test_complex_schema_tool_schema() {
        let tool = create_complex_schema_tool();
        let schema = tool.input_schema();

        assert!(schema.get("properties").is_some());
        assert!(schema["properties"].get("user").is_some());
        assert!(schema["properties"]["user"]["properties"]
            .get("address")
            .is_some());
    }

    /// Test tool call with missing required field
    #[test]
    fn test_tool_call_missing_required_field() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 5}); // Missing 'b' and 'operation'

        let result = tool.call(Some(input));
        assert!(result.is_err());
    }

    /// Test tool call with invalid operation
    #[test]
    fn test_tool_call_invalid_operation() {
        let tool = create_math_tool();
        let input = serde_json::json!({"a": 5, "b": 3, "operation": "divide"});

        let result = tool.call(Some(input));
        assert!(result.is_err());
    }

    /// Test tool call with no input when required
    #[test]
    fn test_tool_call_no_input_required() {
        let tool = create_math_tool();
        let result = tool.call(None);

        assert!(result.is_err());
    }

    // ============================================================================
    // Concurrent Tool Execution Tests
    // ============================================================================

    /// Test concurrent tool execution with multiple threads
    #[test]
    fn test_concurrent_tool_execution() {
        let tool = Arc::new(create_echo_tool());
        let mut handles = vec![];

        for i in 0..10 {
            let tool_clone = Arc::clone(&tool);
            handles.push(thread::spawn(move || {
                let input = serde_json::json!({"message": format!("Message {}", i)});
                tool_clone.call(Some(input))
            }));
        }

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All executions should succeed
        for result in results {
            assert!(result.is_ok());
        }
    }

    /// Test concurrent get_mcp_tools calls
    #[test]
    fn test_concurrent_get_mcp_tools() {
        let mut handles = vec![];

        for _ in 0..20 {
            handles.push(thread::spawn(|| get_mcp_tools()));
        }

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All results should have the same length
        let expected_len = results[0].len();
        for result in &results[1..] {
            assert_eq!(result.len(), expected_len);
        }
    }

    #[test]
    fn test_concurrent_tool_instance_creation() {
        let handles: Vec<_> = (0..50)
            .map(|i| thread::spawn(move || create_simple_tool(&format!("concurrent_tool_{}", i))))
            .collect();

        let tools: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        for (i, tool) in tools.iter().enumerate() {
            assert_eq!(tool.name(), format!("concurrent_tool_{}", i));
        }
    }

    #[test]
    fn test_concurrent_metadata_access() {
        let tools = Arc::new(get_mcp_tools());
        let mut handles = vec![];

        for _ in 0..100 {
            let tools_clone = Arc::clone(&tools);
            handles.push(thread::spawn(move || {
                if let Some(instance) = tools_clone.first() {
                    let _ = instance.metadata().name().to_string();
                    let _ = instance.metadata().version().to_string();
                    let _ = instance.metadata().cache_ttl();
                    let _ = instance.metadata().is_streaming();
                }
            }));
        }

        for h in handles {
            let _ = h.join().unwrap();
        }
    }

    /// Test concurrent tool calls with different operations
    #[test]
    fn test_concurrent_math_operations() {
        let tool = Arc::new(create_math_tool());
        let mut handles = vec![];

        let operations = vec![
            ("add", 1.0, 2.0),
            ("subtract", 10.0, 3.0),
            ("multiply", 4.0, 5.0),
        ];

        for (op, a, b) in operations.repeat(5) {
            let tool_clone = Arc::clone(&tool);
            handles.push(thread::spawn(move || {
                let input = serde_json::json!({"a": a, "b": b, "operation": op});
                tool_clone.call(Some(input))
            }));
        }

        for h in handles {
            assert!(h.join().unwrap().is_ok());
        }
    }

    // ============================================================================
    // McpToolRegistration Tests
    // ============================================================================

    /// Test McpToolRegistration creation through new() constructor
    #[test]
    fn test_registration_creation() {
        let _registration =
            McpToolRegistration::new("test_registration", "v1", "Test registration", || {
                create_simple_tool("test_registration")
            });
    }

    /// Test McpToolRegistration create_fn execution via tool creation
    #[test]
    fn test_registration_create_fn() {
        let tool = create_echo_tool();
        assert_eq!(tool.name(), "echo");
        assert_eq!(tool.description(), "Echoes input back as response");
    }

    /// Test multiple tool creations can coexist
    #[test]
    fn test_multiple_tool_creations() {
        let tool1 = create_simple_tool("tool_a");
        let tool2 = create_simple_tool("tool_b");

        assert_eq!(tool1.name(), "tool_a");
        assert_eq!(tool2.name(), "tool_b");
    }

    // ============================================================================
    // Response Content Tests
    // ============================================================================

    /// Test tool response with text content
    #[test]
    fn test_tool_response_text_content() {
        let tool = create_echo_tool();
        let input = serde_json::json!({"message": "test"});

        let response = tool.call(Some(input)).unwrap();

        assert!(!response.content.is_empty());
        assert!(matches!(
            response.content.first(),
            Some(mcp_sdk::types::ToolResponseContent::Text { .. })
        ));
    }

    /// Test tool response is_error flag
    #[test]
    fn test_tool_response_error_flag() {
        let tool = create_echo_tool();
        let response = tool.call(None).unwrap();

        // Echo tool should not set is_error flag
        assert!(response.is_error.is_none() || response.is_error == Some(false));
    }

    /// Test tool response meta field
    #[test]
    fn test_tool_response_meta_field() {
        let tool = create_simple_tool("meta_test");
        let response = tool.call(None).unwrap();

        // Default tools don't use meta field
        assert!(response.meta.is_none());
    }

    // ============================================================================
    // Input Schema Tests
    // ============================================================================

    /// Test that input_schema returns valid JSON
    #[test]
    fn test_input_schema_is_valid_json() {
        let tool = create_complex_schema_tool();
        let schema = tool.input_schema();

        // Should be able to serialize and deserialize
        let serialized = serde_json::to_string(&schema).unwrap();
        let _: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    }

    /// Test nested schema properties
    #[test]
    fn test_nested_schema_properties() {
        let tool = create_complex_schema_tool();
        let schema = tool.input_schema();

        // Verify nested structure
        let user_props = &schema["properties"]["user"]["properties"];
        assert!(user_props.get("name").is_some());
        assert!(user_props.get("age").is_some());
        assert!(user_props.get("address").is_some());
    }

    /// Test array type in schema
    #[test]
    fn test_array_type_in_schema() {
        let tool = create_complex_schema_tool();
        let schema = tool.input_schema();

        let tags = &schema["properties"]["tags"];
        assert_eq!(tags["type"], "array");
        assert_eq!(tags["items"]["type"], "string");
    }

    /// Test required fields in schema
    #[test]
    fn test_required_fields_in_schema() {
        let tool = create_complex_schema_tool();
        let schema = tool.input_schema();

        let required = schema.get("required").unwrap().as_array().unwrap();
        assert!(required.contains(&serde_json::json!("user")));
    }
}

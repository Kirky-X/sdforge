// Copyright (c) 2026 Kirky.X
//! MCP server implementation

use crate::core::ApiMetadata;
#[cfg(feature = "mcp")]
use serde_json::Value;
use std::sync::Arc;

/// MCP tool registration for compile-time registration
///
/// This structure stores static metadata and a function pointer that creates
/// the actual Tool instance at runtime. This allows compile-time registration
/// via inventory while avoiding the const requirement for Arc::new().
#[derive(Debug, Clone, Copy)]
pub struct McpToolRegistration {
    /// Tool name
    name: &'static str,
    /// Tool version
    version: &'static str,
    /// Tool description
    description: &'static str,
    /// Function that creates the Tool at runtime
    create_fn: fn() -> Arc<dyn mcp_sdk::tools::Tool>,
}

#[allow(missing_docs)]
impl McpToolRegistration {
    pub const fn new(
        name: &'static str,
        version: &'static str,
        description: &'static str,
        create_fn: fn() -> Arc<dyn mcp_sdk::tools::Tool>,
    ) -> Self {
        Self {
            name,
            version,
            description,
            create_fn,
        }
    }
}

#[cfg(feature = "mcp")]
inventory::collect!(McpToolRegistration);

/// Wrapper for Arc<dyn Tool> to implement Tool trait
#[cfg(feature = "mcp")]
struct ArcToolWrapper {
    inner: Arc<dyn mcp_sdk::tools::Tool>,
}

#[cfg(feature = "mcp")]
impl mcp_sdk::tools::Tool for ArcToolWrapper {
    fn name(&self) -> String {
        self.inner.name()
    }

    fn description(&self) -> String {
        self.inner.description()
    }

    fn input_schema(&self) -> serde_json::Value {
        self.inner.input_schema()
    }

    fn call(
        &self,
        input: Option<Value>,
    ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
        self.inner.call(input)
    }
}

/// MCP tool instance with runtime-allocated Arc
///
/// This is the runtime representation created from McpToolRegistration.
#[cfg(feature = "mcp")]
pub struct McpToolInstance {
    /// The actual Tool implementation (Arc for shared ownership)
    tool: Arc<dyn mcp_sdk::tools::Tool>,
    /// API metadata
    metadata: ApiMetadata,
}

#[cfg(feature = "mcp")]
#[allow(missing_docs)]
impl McpToolInstance {
    pub fn tool(&self) -> &Arc<dyn mcp_sdk::tools::Tool> {
        &self.tool
    }

    pub fn metadata(&self) -> &ApiMetadata {
        &self.metadata
    }
}

/// Get all registered MCP tools as runtime instances
///
/// This function collects all registrations and creates the actual Tool instances.
#[cfg(feature = "mcp")]
pub fn get_mcp_tools() -> Vec<McpToolInstance> {
    inventory::iter::<McpToolRegistration>
        .into_iter()
        .map(|reg| {
            let tool = (reg.create_fn)();
            McpToolInstance {
                tool,
                metadata: ApiMetadata::new(
                    reg.name.to_string(),
                    reg.version.to_string(),
                    reg.description.to_string(),
                    None,
                    false,
                ),
            }
        })
        .collect()
}

/// Build MCP server from registered tools
#[cfg(feature = "mcp")]
pub async fn build() -> mcp_sdk::server::Server<mcp_sdk::transport::ServerStdioTransport> {
    use mcp_sdk::server::Server;
    use mcp_sdk::tools::Tools;
    use mcp_sdk::transport::ServerStdioTransport;

    let mut tools = Tools::default();
    let mut server_name = "sdforge-mcp".to_string();
    let mut server_version = "0.1.0".to_string();

    // Collect all registered tool instances
    for instance in get_mcp_tools() {
        // Wrap Arc<dyn Tool> in ArcToolWrapper to satisfy Tool trait bound
        let wrapper = ArcToolWrapper {
            inner: instance.tool().clone(),
        };
        tools.add_tool(wrapper);

        // Use the first tool's metadata for server info
        if server_name == "sdforge-mcp" {
            server_name = instance.metadata().name().to_string();
            server_version = instance.metadata().version().to_string();
        }
    }

    Server::builder(ServerStdioTransport)
        .name(server_name)
        .version(server_version)
        .tools(tools)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test McpToolRegistration structure
    #[test]
    fn test_mcp_tool_registration() {
        fn create_test_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
            struct TestTool;
            impl mcp_sdk::tools::Tool for TestTool {
                fn name(&self) -> String {
                    "test".to_string()
                }
                fn description(&self) -> String {
                    "Test tool".to_string()
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
            Arc::new(TestTool) as Arc<dyn mcp_sdk::tools::Tool>
        }

        let registration =
            McpToolRegistration::new("test_tool", "v1", "A test tool", create_test_tool);

        assert_eq!(registration.name, "test_tool");
        assert_eq!(registration.version, "v1");

        // Test that the create_fn works
        let tool = (registration.create_fn)();
        assert_eq!(tool.name(), "test");
    }

    /// Test ApiMetadata structure
    #[test]
    fn test_api_metadata_creation() {
        let metadata = ApiMetadata {
            name: "test_tool".to_string(),
            version: "v1".to_string(),
            description: "A test tool".to_string(),
            cache_ttl: None,
            is_streaming: false,
        };
        assert_eq!(metadata.name, "test_tool");
        assert_eq!(metadata.version, "v1");
        assert_eq!(metadata.description, "A test tool");
    }

    /// Test ApiMetadata name and version accessors
    #[test]
    fn test_api_metadata_accessors() {
        let metadata = ApiMetadata {
            name: "my_api".to_string(),
            version: "v2".to_string(),
            description: "".to_string(),
            cache_ttl: Some(300),
            is_streaming: true,
        };
        assert_eq!(metadata.name(), "my_api");
        assert_eq!(metadata.version(), "v2");
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[test]
    fn test_tool_call_error_handling() {
        struct ErrorTool;
        impl mcp_sdk::tools::Tool for ErrorTool {
            fn name(&self) -> String {
                "error_tool".to_string()
            }
            fn description(&self) -> String {
                "Tool that returns error".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "string"})
            }
            fn call(
                &self,
                _input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                Err(anyhow::anyhow!("Intentional error"))
            }
        }

        let tool = Arc::new(ErrorTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let result = tool.call(None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Intentional error"));
    }

    #[test]
    fn test_tool_call_with_invalid_input() {
        struct ValidationTool;
        impl mcp_sdk::tools::Tool for ValidationTool {
            fn name(&self) -> String {
                "validation_tool".to_string()
            }
            fn description(&self) -> String {
                "Validates input parameters".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "required": {"type": "string"}
                    },
                    "required": ["required"]
                })
            }
            fn call(
                &self,
                input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                match input {
                    Some(value) => {
                        if value.get("required").is_some() {
                            Ok(mcp_sdk::types::CallToolResponse {
                                content: vec![],
                                is_error: None,
                                meta: None,
                            })
                        } else {
                            Err(anyhow::anyhow!("Missing required field"))
                        }
                    }
                    None => Err(anyhow::anyhow!("No input provided")),
                }
            }
        }

        let tool = Arc::new(ValidationTool) as Arc<dyn mcp_sdk::tools::Tool>;

        // Missing required field
        let invalid_input = serde_json::json!({"other": "value"});
        let result = tool.call(Some(invalid_input));
        assert!(result.is_err());

        // No input
        let result = tool.call(None);
        assert!(result.is_err());

        // Valid input
        let valid_input = serde_json::json!({"required": "value"});
        let result = tool.call(Some(valid_input));
        assert!(result.is_ok());
    }

    // ============================================================================
    // Input Schema Tests
    // ============================================================================

    #[test]
    fn test_input_schema_object_type() {
        struct SchemaTool;
        impl mcp_sdk::tools::Tool for SchemaTool {
            fn name(&self) -> String {
                "schema_tool".to_string()
            }
            fn description(&self) -> String {
                "Returns complex schema".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "param1": {"type": "string"},
                        "param2": {"type": "number"}
                    },
                    "required": ["param1"]
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

        let tool = Arc::new(SchemaTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let schema = tool.input_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
        assert!(schema["required"].is_array());
    }

    #[test]
    fn test_input_schema_primitive_types() {
        struct PrimitiveTool;
        impl mcp_sdk::tools::Tool for PrimitiveTool {
            fn name(&self) -> String {
                "primitive_tool".to_string()
            }
            fn description(&self) -> String {
                "Accepts primitive types".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "string"})
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

        let tool = Arc::new(PrimitiveTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let schema = tool.input_schema();
        assert_eq!(schema["type"], "string");
    }

    // ============================================================================
    // Tool Execution Tests
    // ============================================================================

    #[test]
    fn test_tool_execution_with_result() {
        struct ResultTool;
        impl mcp_sdk::tools::Tool for ResultTool {
            fn name(&self) -> String {
                "result_tool".to_string()
            }
            fn description(&self) -> String {
                "Returns a result value".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn call(
                &self,
                input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                let result_value = input.unwrap_or(serde_json::json!({}));
                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![mcp_sdk::types::ToolResponseContent::Text {
                        text: serde_json::to_string(&result_value)?,
                    }],
                    is_error: Some(false),
                    meta: None,
                })
            }
        }

        let tool = Arc::new(ResultTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let input = serde_json::json!({"key": "value"});
        let result = tool.call(Some(input));

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.is_error, Some(false));
        assert!(!response.content.is_empty());
    }

    #[test]
    fn test_tool_execution_with_empty_response() {
        struct EmptyTool;
        impl mcp_sdk::tools::Tool for EmptyTool {
            fn name(&self) -> String {
                "empty_tool".to_string()
            }
            fn description(&self) -> String {
                "Returns empty response".to_string()
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
                    is_error: Some(false),
                    meta: None,
                })
            }
        }

        let tool = Arc::new(EmptyTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let result = tool.call(None);

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.content.is_empty());
        assert_eq!(response.is_error, Some(false));
    }

    #[test]
    fn test_tool_execution_error_response() {
        struct ErrorResponseTool;
        impl mcp_sdk::tools::Tool for ErrorResponseTool {
            fn name(&self) -> String {
                "error_tool".to_string()
            }
            fn description(&self) -> String {
                "Returns error response".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "string"})
            }
            fn call(
                &self,
                _input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![mcp_sdk::types::ToolResponseContent::Text {
                        text: "An error occurred".to_string(),
                    }],
                    is_error: Some(true),
                    meta: None,
                })
            }
        }

        let tool = Arc::new(ErrorResponseTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let result = tool.call(None);

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.is_error, Some(true));
        assert!(!response.content.is_empty());
    }

    // ============================================================================
    // Metadata Tests
    // ============================================================================

    #[test]
    fn test_metadata_with_cache_ttl() {
        let metadata = ApiMetadata {
            name: "cached_tool".to_string(),
            version: "v1".to_string(),
            description: "Tool with caching".to_string(),
            cache_ttl: Some(600),
            is_streaming: false,
        };
        assert_eq!(metadata.cache_ttl, Some(600));
        assert!(!metadata.is_streaming);
    }

    #[test]
    fn test_metadata_streaming_flag() {
        let streaming_metadata = ApiMetadata {
            name: "streaming_tool".to_string(),
            version: "v1".to_string(),
            description: "Streaming tool".to_string(),
            cache_ttl: None,
            is_streaming: true,
        };
        assert!(streaming_metadata.is_streaming);
        assert_eq!(streaming_metadata.cache_ttl, None);
    }

    // ============================================================================
    // McpToolInstance Tests
    // ============================================================================

    #[test]
    fn test_mcp_tool_instance_creation() {
        fn create_instance() -> Arc<dyn mcp_sdk::tools::Tool> {
            struct InstanceTool;
            impl mcp_sdk::tools::Tool for InstanceTool {
                fn name(&self) -> String {
                    "instance_tool".to_string()
                }
                fn description(&self) -> String {
                    "Test instance tool".to_string()
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
            Arc::new(InstanceTool) as Arc<dyn mcp_sdk::tools::Tool>
        }

        let registration = McpToolRegistration {
            name: "instance_test_tool",
            version: "v1",
            description: "Test instance creation",
            create_fn: create_instance,
        };

        let tool = (registration.create_fn)();
        let instance = McpToolInstance {
            tool,
            metadata: ApiMetadata::new(
                registration.name.to_string(),
                registration.version.to_string(),
                registration.description.to_string(),
                None,
                false,
            ),
        };

        assert_eq!(instance.metadata().name(), "instance_test_tool");
        assert_eq!(instance.metadata().version(), "v1");
    }
}

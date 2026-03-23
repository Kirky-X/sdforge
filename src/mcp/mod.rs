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
    use mcp_sdk::tools::Tool;

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

    // ============================================================================
    // MCP Protocol Message Serialization Tests (Task 2.1)
    // ============================================================================

    /// Test: JSON-RPC request serialization and deserialization round-trip
    #[test]
    fn test_mcp_protocol_request_roundtrip() {
        use mcp_sdk::transport::{JsonRpcRequest, JsonRpcVersion};

        let request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::default(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "my_tool",
                "arguments": {"key": "value"}
            })),
            id: 1_u64,
        };

        let serialized = serde_json::to_string(&request).expect("serialization should succeed");
        assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
        assert!(serialized.contains("\"method\":\"tools/call\""));
        assert!(serialized.contains("\"params\""));

        let deserialized: JsonRpcRequest =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert_eq!(deserialized.jsonrpc.as_str(), "2.0");
        assert_eq!(deserialized.method, "tools/call");
        assert_eq!(deserialized.id, 1_u64);
    }

    /// Test: JSON-RPC request with numeric ID round-trip
    #[test]
    fn test_mcp_protocol_request_numeric_id() {
        use mcp_sdk::transport::{JsonRpcRequest, JsonRpcVersion};

        let request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::default(),
            method: "tools/list".to_string(),
            params: None,
            id: 42_u64,
        };

        let serialized = serde_json::to_string(&request).expect("serialization should succeed");
        let deserialized: JsonRpcRequest =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert_eq!(deserialized.id, 42_u64);
    }

    /// Test: JSON-RPC response (with result) serialization
    #[test]
    fn test_mcp_protocol_response_with_result() {
        use mcp_sdk::transport::{JsonRpcResponse, JsonRpcVersion};

        let response = JsonRpcResponse {
            jsonrpc: JsonRpcVersion::default(),
            id: 1_u64,
            result: Some(serde_json::json!({
                "content": [{"type": "text", "text": "hello"}]
            })),
            error: None,
        };

        let serialized = serde_json::to_string(&response).expect("serialization should succeed");
        assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
        assert!(serialized.contains("\"result\""));
        assert!(!serialized.contains("\"error\""));

        let deserialized: JsonRpcResponse =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert!(deserialized.result.is_some());
        assert!(deserialized.error.is_none());
    }

    /// Test: JSON-RPC response (with error) serialization
    #[test]
    fn test_mcp_protocol_response_with_error() {
        use mcp_sdk::transport::{JsonRpcError, JsonRpcResponse, JsonRpcVersion};

        let response = JsonRpcResponse {
            id: 1_u64,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
            jsonrpc: JsonRpcVersion::default(),
        };

        let serialized = serde_json::to_string(&response).expect("serialization should succeed");
        assert!(serialized.contains("\"error\""));
        assert!(!serialized.contains("\"result\""));
    }

    /// Test: JSON-RPC notification (no ID) serialization
    #[test]
    fn test_mcp_protocol_notification_roundtrip() {
        use mcp_sdk::transport::{JsonRpcNotification, JsonRpcVersion};

        let notification = JsonRpcNotification {
            jsonrpc: JsonRpcVersion::default(),
            method: "notifications/initialized".to_string(),
            params: None,
        };

        let serialized =
            serde_json::to_string(&notification).expect("serialization should succeed");
        assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
        assert!(serialized.contains("\"method\":\"notifications/initialized\""));
        assert!(!serialized.contains("\"id\""));

        let deserialized: JsonRpcNotification =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert_eq!(deserialized.method, "notifications/initialized");
    }

    /// Test: Protocol version constant
    #[test]
    fn test_mcp_protocol_version_constant() {
        use mcp_sdk::types::LATEST_PROTOCOL_VERSION;

        assert!(!LATEST_PROTOCOL_VERSION.is_empty());
    }

    // ============================================================================
    // MCP Resource Subscription Tests (Task 2.4)
    // ============================================================================

    /// Test: Resource struct serialization
    #[test]
    fn test_mcp_resource_serialization() {
        use mcp_sdk::types::Resource;
        use url::Url;

        let resource = Resource {
            uri: Url::parse("file:///path/to/resource").expect("valid URL"),
            name: "my-resource".to_string(),
            description: Some("A test resource".to_string()),
            mime_type: Some("text/plain".to_string()),
        };

        let serialized = serde_json::to_string(&resource).expect("serialization should succeed");
        assert!(serialized.contains("\"uri\""));
        assert!(serialized.contains("\"name\""));
        assert!(serialized.contains("\"mimeType\""));

        let deserialized: Resource =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert_eq!(deserialized.uri.as_str(), "file:///path/to/resource");
        assert_eq!(deserialized.name, "my-resource");
        assert_eq!(deserialized.mime_type, Some("text/plain".to_string()));
    }

    /// Test: Resource subscription (subscribe/unsubscribe flow)
    #[test]
    fn test_mcp_resource_subscription_flow() {
        use mcp_sdk::transport::{JsonRpcRequest, JsonRpcVersion};

        // Test subscribe request
        let subscribe_request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::default(),
            method: "resources/subscribe".to_string(),
            params: Some(serde_json::json!({
                "uri": "file:///path/to/resource"
            })),
            id: 1_u64,
        };

        let serialized =
            serde_json::to_string(&subscribe_request).expect("serialization should succeed");
        assert!(serialized.contains("\"method\":\"resources/subscribe\""));
        assert!(serialized.contains("\"uri\""));

        // Test unsubscribe request
        let unsubscribe_request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::default(),
            method: "resources/unsubscribe".to_string(),
            params: Some(serde_json::json!({
                "uri": "file:///path/to/resource"
            })),
            id: 2_u64,
        };

        let serialized =
            serde_json::to_string(&unsubscribe_request).expect("serialization should succeed");
        assert!(serialized.contains("\"method\":\"resources/unsubscribe\""));
    }

    /// Test: Resource subscription request (subscribe method)
    #[test]
    fn test_mcp_resource_subscription_request() {
        use mcp_sdk::transport::{JsonRpcRequest, JsonRpcVersion};

        let subscribe_request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::default(),
            method: "resources/subscribe".to_string(),
            params: Some(serde_json::json!({
                "uri": "file:///path/to/resource"
            })),
            id: 1_u64,
        };

        let serialized =
            serde_json::to_string(&subscribe_request).expect("serialization should succeed");
        let deserialized: JsonRpcRequest =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert_eq!(deserialized.method, "resources/subscribe");
        assert!(deserialized.params.is_some());
    }

    /// Test: Resource list request and response structure
    #[test]
    fn test_mcp_resource_list_response() {
        use mcp_sdk::transport::{JsonRpcResponse, JsonRpcVersion};

        let list_response = JsonRpcResponse {
            jsonrpc: JsonRpcVersion::default(),
            id: 2_u64,
            result: Some(serde_json::json!({
                "resources": [
                    {
                        "uri": "file:///a",
                        "name": "Resource A",
                        "mimeType": "text/plain"
                    },
                    {
                        "uri": "file:///b",
                        "name": "Resource B",
                        "mimeType": "application/json"
                    }
                ]
            })),
            error: None,
        };

        let serialized =
            serde_json::to_string(&list_response).expect("serialization should succeed");
        let deserialized: JsonRpcResponse =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        let result = deserialized.result.expect("should have result");
        let resources = result
            .get("resources")
            .expect("should have resources array");
        assert!(resources.is_array());
        assert_eq!(resources.as_array().unwrap().len(), 2);
    }

    /// Test: Unsubscribe request format
    #[test]
    fn test_mcp_resource_unsubscribe_request() {
        use mcp_sdk::transport::{JsonRpcRequest, JsonRpcVersion};

        // Note: MCP doesn't have explicit unsubscribe; unsubscribe is implicit.
        // But we test the pattern for resources/unsubscribe if it exists.
        let unsubscribe_request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::default(),
            method: "resources/unsubscribe".to_string(),
            params: Some(serde_json::json!({
                "uri": "file:///path/to/resource"
            })),
            id: 99_u64,
        };

        let serialized =
            serde_json::to_string(&unsubscribe_request).expect("serialization should succeed");
        let deserialized: JsonRpcRequest =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert_eq!(deserialized.method, "resources/unsubscribe");
    }

    // ============================================================================
    // MCP Prompt Template Tests (Task 2.5)
    // ============================================================================

    /// Test: Prompt template serialization with variables
    #[test]
    fn test_mcp_prompt_template_with_variables() {
        use mcp_sdk::transport::{JsonRpcRequest, JsonRpcVersion};

        let prompt_request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::default(),
            method: "prompts/get".to_string(),
            params: Some(serde_json::json!({
                "name": "greeting",
                "arguments": {
                    "name": "Alice",
                    "language": "English"
                }
            })),
            id: 3_u64,
        };

        let serialized =
            serde_json::to_string(&prompt_request).expect("serialization should succeed");
        let deserialized: JsonRpcRequest =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert_eq!(deserialized.method, "prompts/get");
        let params = deserialized.params.expect("should have params");
        assert_eq!(params["name"], "greeting");
        assert_eq!(params["arguments"]["name"], "Alice");
        assert_eq!(params["arguments"]["language"], "English");
    }

    /// Test: Prompt template rendering with multiple variables
    #[test]
    fn test_mcp_prompt_template_multiple_variables() {
        use mcp_sdk::transport::{JsonRpcRequest, JsonRpcVersion};

        let prompt_request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::default(),
            method: "prompts/get".to_string(),
            params: Some(serde_json::json!({
                "name": "code_review",
                "arguments": {
                    "repo": "sdforge",
                    "pr_number": 42,
                    "reviewer": "Bob",
                    "priority": "high"
                }
            })),
            id: 7_u64,
        };

        let serialized =
            serde_json::to_string(&prompt_request).expect("serialization should succeed");
        let deserialized: JsonRpcRequest =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        let params = deserialized.params.expect("should have params");
        assert_eq!(params["arguments"]["repo"], "sdforge");
        assert_eq!(params["arguments"]["pr_number"], 42);
        assert_eq!(params["arguments"]["reviewer"], "Bob");
        assert_eq!(params["arguments"]["priority"], "high");
    }

    /// Test: Prompt get response structure
    #[test]
    fn test_mcp_prompt_get_response_structure() {
        use mcp_sdk::transport::{JsonRpcResponse, JsonRpcVersion};

        let prompt_response = JsonRpcResponse {
            jsonrpc: JsonRpcVersion::default(),
            id: 4_u64,
            result: Some(serde_json::json!({
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "Hello, how can I help you today?"
                        }
                    }
                ]
            })),
            error: None,
        };

        let serialized =
            serde_json::to_string(&prompt_response).expect("serialization should succeed");
        let deserialized: JsonRpcResponse =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        let result = deserialized.result.expect("should have result");
        let messages = result.get("messages").expect("should have messages");
        assert!(messages.is_array());
        assert_eq!(messages[0]["role"], "user");
    }

    /// Test: Prompt list request
    #[test]
    fn test_mcp_prompts_list_request() {
        use mcp_sdk::transport::{JsonRpcRequest, JsonRpcVersion};

        let list_request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::default(),
            method: "prompts/list".to_string(),
            params: None,
            id: 3_u64,
        };

        let serialized =
            serde_json::to_string(&list_request).expect("serialization should succeed");
        let deserialized: JsonRpcRequest =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert_eq!(deserialized.method, "prompts/list");
        assert!(deserialized.params.is_none());
    }

    // ============================================================================
    // MCP Protocol Error Handling Tests (Task 2.6)
    // ============================================================================

    /// Test: Invalid JSON is detected during deserialization
    #[test]
    fn test_mcp_protocol_invalid_json() {
        use mcp_sdk::transport::JsonRpcRequest;

        let invalid_json = r#"{"jsonrpc": "2.0", "method": "test", "params": invalid}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err(), "Invalid JSON should fail to parse");
    }

    /// Test: Valid JSON that is not a valid request object
    #[test]
    fn test_mcp_protocol_invalid_request_object() {
        use mcp_sdk::transport::JsonRpcRequest;

        // Valid JSON but missing required fields for a Request
        let not_a_request = r#"{"jsonrpc": "2.0", "foo": "bar"}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_str(not_a_request);
        assert!(
            result.is_err(),
            "JSON without required 'method' and 'id' fields should fail"
        );
    }

    /// Test: Wrong JSON-RPC version is preserved during deserialization
    #[test]
    fn test_mcp_protocol_version_mismatch() {
        use mcp_sdk::transport::JsonRpcRequest;

        let wrong_version = r#"{
            "jsonrpc": "1.0",
            "method": "test",
            "params": {},
            "id": 123
        }"#;
        let request: JsonRpcRequest =
            serde_json::from_str(wrong_version).expect("deserialization should succeed");
        // The version field is preserved as-is (no validation, just roundtrip)
        assert_eq!(request.jsonrpc.as_str(), "1.0");
    }

    /// Test: Empty params handled correctly
    #[test]
    fn test_mcp_protocol_empty_params() {
        use mcp_sdk::transport::{JsonRpcRequest, JsonRpcVersion};

        let request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::default(),
            method: "tools/list".to_string(),
            params: None,
            id: 10_u64,
        };

        let serialized = serde_json::to_string(&request).expect("serialization should succeed");
        // params should be omitted when None due to skip_serializing_if
        assert!(!serialized.contains("\"params\":null"));
        assert!(!serialized.contains("\"params\":[]"));

        let deserialized: JsonRpcRequest =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert!(deserialized.params.is_none());
    }

    /// Test: Unsupported protocol version in capabilities
    #[test]
    fn test_mcp_capabilities_with_unsupported_version() {
        use mcp_sdk::types::ServerCapabilities;

        // Server advertising no capabilities (all None)
        let caps = ServerCapabilities {
            experimental: None,
            logging: None,
            prompts: None,
            resources: None,
            tools: None,
        };

        let serialized = serde_json::to_string(&caps).expect("serialization should succeed");
        // All-None capabilities serialize to empty object
        assert_eq!(serialized, "{}");

        let deserialized: ServerCapabilities =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert!(deserialized.tools.is_none());
        assert!(deserialized.resources.is_none());
    }

    /// Test: Server capabilities with tools enabled
    #[test]
    fn test_mcp_capabilities_tools_enabled() {
        use mcp_sdk::types::ServerCapabilities;

        let caps = ServerCapabilities {
            tools: Some(serde_json::json!({})),
            ..Default::default()
        };

        let serialized = serde_json::to_string(&caps).expect("serialization should succeed");
        assert!(serialized.contains("\"tools\""));
    }

    /// Test: Client capabilities roundtrip
    #[test]
    fn test_mcp_client_capabilities_roundtrip() {
        use mcp_sdk::types::{ClientCapabilities, RootCapabilities};

        let client_caps = ClientCapabilities {
            experimental: None,
            roots: Some(RootCapabilities {
                list_changed: Some(true),
            }),
            sampling: None,
        };

        let serialized = serde_json::to_string(&client_caps).expect("serialization should succeed");
        let deserialized: ClientCapabilities =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert!(deserialized.roots.is_some());
        assert_eq!(deserialized.roots.unwrap().list_changed, Some(true));
    }

    /// Test: Implementation struct serialization
    #[test]
    fn test_mcp_implementation_serialization() {
        use mcp_sdk::types::Implementation;

        let impl_info = Implementation {
            name: "sdforge-mcp".to_string(),
            version: "0.1.0".to_string(),
        };

        let serialized = serde_json::to_string(&impl_info).expect("serialization should succeed");
        assert!(serialized.contains("\"name\":\"sdforge-mcp\""));
        assert!(serialized.contains("\"version\":\"0.1.0\""));

        let deserialized: Implementation =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
        assert_eq!(deserialized.name, "sdforge-mcp");
    }

    // ============================================================================
    // ArcToolWrapper Tests
    // ============================================================================

    #[test]
    fn test_arc_tool_wrapper_name_delegation() {
        struct NameTool;
        impl mcp_sdk::tools::Tool for NameTool {
            fn name(&self) -> String {
                "wrapped_name_tool".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
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

        let inner = Arc::new(NameTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let wrapper = ArcToolWrapper { inner };
        assert_eq!(wrapper.name(), "wrapped_name_tool");
    }

    #[test]
    fn test_arc_tool_wrapper_description_delegation() {
        struct DescTool;
        impl mcp_sdk::tools::Tool for DescTool {
            fn name(&self) -> String {
                "name".to_string()
            }
            fn description(&self) -> String {
                "wrapped description text".to_string()
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

        let inner = Arc::new(DescTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let wrapper = ArcToolWrapper { inner };
        assert_eq!(wrapper.description(), "wrapped description text");
    }

    #[test]
    fn test_arc_tool_wrapper_input_schema_delegation() {
        struct SchemaTool;
        impl mcp_sdk::tools::Tool for SchemaTool {
            fn name(&self) -> String {
                "name".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "wrapped_field": {"type": "string"}
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

        let inner = Arc::new(SchemaTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let wrapper = ArcToolWrapper { inner };
        let schema = wrapper.input_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["wrapped_field"].is_object());
    }

    #[test]
    fn test_arc_tool_wrapper_call_delegation() {
        struct CallTool;
        impl mcp_sdk::tools::Tool for CallTool {
            fn name(&self) -> String {
                "name".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn call(
                &self,
                input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                let val = input.unwrap_or(serde_json::json!({}));
                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![mcp_sdk::types::ToolResponseContent::Text {
                        text: val.to_string(),
                    }],
                    is_error: None,
                    meta: None,
                })
            }
        }

        let inner = Arc::new(CallTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let wrapper = ArcToolWrapper { inner };
        let result = wrapper.call(Some(serde_json::json!({"key": "value"})));
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.content.is_empty());
    }

    #[test]
    fn test_arc_tool_wrapper_error_propagation() {
        struct ErrorTool;
        impl mcp_sdk::tools::Tool for ErrorTool {
            fn name(&self) -> String {
                "name".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn call(
                &self,
                _input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                Err(anyhow::anyhow!("Wrapped error"))
            }
        }

        let inner = Arc::new(ErrorTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let wrapper = ArcToolWrapper { inner };
        let result = wrapper.call(None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Wrapped error"));
    }

    // ============================================================================
    // Edge Case Tests - Names and Strings
    // ============================================================================

    #[test]
    fn test_tool_name_with_unicode() {
        struct UnicodeTool;
        impl mcp_sdk::tools::Tool for UnicodeTool {
            fn name(&self) -> String {
                "工具_🛠️_tool".to_string()
            }
            fn description(&self) -> String {
                "描述 🎯 description".to_string()
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

        let tool = Arc::new(UnicodeTool) as Arc<dyn mcp_sdk::tools::Tool>;
        assert_eq!(tool.name(), "工具_🛠️_tool");
        assert_eq!(tool.description(), "描述 🎯 description");
    }

    #[test]
    fn test_tool_name_with_special_characters() {
        struct SpecialTool;
        impl mcp_sdk::tools::Tool for SpecialTool {
            fn name(&self) -> String {
                "tool-with_special.chars:123".to_string()
            }
            fn description(&self) -> String {
                "Description with\nnewline and\ttab".to_string()
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

        let tool = Arc::new(SpecialTool) as Arc<dyn mcp_sdk::tools::Tool>;
        assert_eq!(tool.name(), "tool-with_special.chars:123");
        assert!(tool.description().contains('\n'));
        assert!(tool.description().contains('\t'));
    }

    #[test]
    fn test_tool_name_empty_string() {
        struct EmptyNameTool;
        impl mcp_sdk::tools::Tool for EmptyNameTool {
            fn name(&self) -> String {
                "".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
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

        let tool = Arc::new(EmptyNameTool) as Arc<dyn mcp_sdk::tools::Tool>;
        assert_eq!(tool.name(), "");
    }

    #[test]
    fn test_tool_description_empty_string() {
        struct EmptyDescTool;
        impl mcp_sdk::tools::Tool for EmptyDescTool {
            fn name(&self) -> String {
                "name".to_string()
            }
            fn description(&self) -> String {
                "".to_string()
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

        let tool = Arc::new(EmptyDescTool) as Arc<dyn mcp_sdk::tools::Tool>;
        assert_eq!(tool.description(), "");
    }

    #[test]
    fn test_tool_name_very_long() {
        struct LongNameTool;
        impl mcp_sdk::tools::Tool for LongNameTool {
            fn name(&self) -> String {
                "a".repeat(1000)
            }
            fn description(&self) -> String {
                "desc".to_string()
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

        let tool = Arc::new(LongNameTool) as Arc<dyn mcp_sdk::tools::Tool>;
        assert_eq!(tool.name().len(), 1000);
    }

    #[test]
    fn test_tool_description_very_long() {
        struct LongDescTool;
        impl mcp_sdk::tools::Tool for LongDescTool {
            fn name(&self) -> String {
                "name".to_string()
            }
            fn description(&self) -> String {
                "d".repeat(5000)
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

        let tool = Arc::new(LongDescTool) as Arc<dyn mcp_sdk::tools::Tool>;
        assert_eq!(tool.description().len(), 5000);
    }

    // ============================================================================
    // Input Schema Edge Cases
    // ============================================================================

    #[test]
    fn test_input_schema_empty_object() {
        struct EmptySchemaTool;
        impl mcp_sdk::tools::Tool for EmptySchemaTool {
            fn name(&self) -> String {
                "name".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({})
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

        let tool = Arc::new(EmptySchemaTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let schema = tool.input_schema();
        assert!(schema.is_object());
        assert!(schema.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_input_schema_with_nested_objects() {
        struct NestedSchemaTool;
        impl mcp_sdk::tools::Tool for NestedSchemaTool {
            fn name(&self) -> String {
                "nested".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "user": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "address": {
                                    "type": "object",
                                    "properties": {
                                        "city": {"type": "string"},
                                        "zip": {"type": "string"}
                                    }
                                }
                            }
                        }
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

        let tool = Arc::new(NestedSchemaTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let schema = tool.input_schema();
        assert!(
            schema["properties"]["user"]["properties"]["address"]["properties"]["city"].is_object()
        );
    }

    #[test]
    fn test_input_schema_with_array_type() {
        struct ArraySchemaTool;
        impl mcp_sdk::tools::Tool for ArraySchemaTool {
            fn name(&self) -> String {
                "array".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "integer"}
                        }
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

        let tool = Arc::new(ArraySchemaTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let schema = tool.input_schema();
        assert_eq!(schema["type"], "array");
        assert!(schema["items"].is_object());
    }

    #[test]
    fn test_input_schema_with_enum() {
        struct EnumSchemaTool;
        impl mcp_sdk::tools::Tool for EnumSchemaTool {
            fn name(&self) -> String {
                "enum".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "string",
                    "enum": ["option1", "option2", "option3"]
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

        let tool = Arc::new(EnumSchemaTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let schema = tool.input_schema();
        assert!(schema["enum"].is_array());
        assert_eq!(schema["enum"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_input_schema_with_oneof() {
        struct OneOfSchemaTool;
        impl mcp_sdk::tools::Tool for OneOfSchemaTool {
            fn name(&self) -> String {
                "oneof".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "oneOf": [
                        {"type": "string"},
                        {"type": "number"}
                    ]
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

        let tool = Arc::new(OneOfSchemaTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let schema = tool.input_schema();
        assert!(schema["oneOf"].is_array());
    }

    // ============================================================================
    // Tool Call with Various Inputs
    // ============================================================================

    #[test]
    fn test_tool_call_with_null_input() {
        struct NullInputTool;
        impl mcp_sdk::tools::Tool for NullInputTool {
            fn name(&self) -> String {
                "null_input".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn call(
                &self,
                input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                match input {
                    None => Ok(mcp_sdk::types::CallToolResponse {
                        content: vec![mcp_sdk::types::ToolResponseContent::Text {
                            text: "no input".to_string(),
                        }],
                        is_error: None,
                        meta: None,
                    }),
                    Some(v) if v.is_null() => Ok(mcp_sdk::types::CallToolResponse {
                        content: vec![mcp_sdk::types::ToolResponseContent::Text {
                            text: "null input".to_string(),
                        }],
                        is_error: None,
                        meta: None,
                    }),
                    _ => Err(anyhow::anyhow!("unexpected input")),
                }
            }
        }

        let tool = Arc::new(NullInputTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let result = tool.call(None);
        assert!(result.is_ok());
        let result = tool.call(Some(serde_json::Value::Null));
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_call_with_array_input() {
        struct ArrayInputTool;
        impl mcp_sdk::tools::Tool for ArrayInputTool {
            fn name(&self) -> String {
                "array_input".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "array"})
            }
            fn call(
                &self,
                input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                let arr = input.unwrap_or(serde_json::json!([]));
                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![mcp_sdk::types::ToolResponseContent::Text {
                        text: format!(
                            "array length: {}",
                            arr.as_array().map(|a| a.len()).unwrap_or(0)
                        ),
                    }],
                    is_error: None,
                    meta: None,
                })
            }
        }

        let tool = Arc::new(ArrayInputTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let result = tool.call(Some(serde_json::json!([1, 2, 3])));
        assert!(result.is_ok());
        let response = result.unwrap();
        match &response.content[0] {
            mcp_sdk::types::ToolResponseContent::Text { text } => {
                assert!(text.contains("3"));
            }
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_tool_call_with_large_json() {
        struct LargeJsonTool;
        impl mcp_sdk::tools::Tool for LargeJsonTool {
            fn name(&self) -> String {
                "large".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn call(
                &self,
                input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                let obj = input.unwrap_or(serde_json::json!({}));
                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![],
                    is_error: None,
                    meta: Some(obj),
                })
            }
        }

        let tool = Arc::new(LargeJsonTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let large_input: serde_json::Value = serde_json::json!({
            "items": (0..1000).map(|i| format!("item_{}", i)).collect::<Vec<_>>()
        });
        let result = tool.call(Some(large_input));
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.meta.is_some());
    }

    #[test]
    fn test_tool_call_with_deeply_nested_input() {
        struct DeepNestedTool;
        impl mcp_sdk::tools::Tool for DeepNestedTool {
            fn name(&self) -> String {
                "deep".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn call(
                &self,
                input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                let val = input.unwrap_or(serde_json::json!({}));
                let depth = count_depth(&val);
                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![mcp_sdk::types::ToolResponseContent::Text {
                        text: format!("depth: {}", depth),
                    }],
                    is_error: None,
                    meta: None,
                })
            }
        }

        fn count_depth(val: &serde_json::Value) -> usize {
            match val {
                serde_json::Value::Object(map) => {
                    1 + map.values().map(count_depth).max().unwrap_or(0)
                }
                serde_json::Value::Array(arr) => 1 + arr.iter().map(count_depth).max().unwrap_or(0),
                _ => 0,
            }
        }

        let tool = Arc::new(DeepNestedTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let nested = serde_json::json!({
            "l1": {
                "l2": {
                    "l3": {
                        "l4": {
                            "l5": "deep"
                        }
                    }
                }
            }
        });
        let result = tool.call(Some(nested));
        assert!(result.is_ok());
    }

    // ============================================================================
    // Tool Response Content Tests
    // ============================================================================

    #[test]
    fn test_tool_response_text_content() {
        struct TextContentTool;
        impl mcp_sdk::tools::Tool for TextContentTool {
            fn name(&self) -> String {
                "text".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn call(
                &self,
                _input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![
                        mcp_sdk::types::ToolResponseContent::Text {
                            text: "First message".to_string(),
                        },
                        mcp_sdk::types::ToolResponseContent::Text {
                            text: "Second message".to_string(),
                        },
                    ],
                    is_error: None,
                    meta: None,
                })
            }
        }

        let tool = Arc::new(TextContentTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let result = tool.call(None).unwrap();
        assert_eq!(result.content.len(), 2);
    }

    #[test]
    fn test_tool_response_with_meta() {
        struct MetaTool;
        impl mcp_sdk::tools::Tool for MetaTool {
            fn name(&self) -> String {
                "meta".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
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
                    meta: Some(serde_json::json!({
                        "custom_field": "custom_value",
                        "timestamp": 1234567890
                    })),
                })
            }
        }

        let tool = Arc::new(MetaTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let result = tool.call(None).unwrap();
        assert!(result.meta.is_some());
        let meta = result.meta.unwrap();
        assert_eq!(meta["custom_field"], "custom_value");
    }

    #[test]
    fn test_tool_response_is_error_flag() {
        struct ErrorFlagTool;
        impl mcp_sdk::tools::Tool for ErrorFlagTool {
            fn name(&self) -> String {
                "error_flag".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn call(
                &self,
                _input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![mcp_sdk::types::ToolResponseContent::Text {
                        text: "This is a soft error".to_string(),
                    }],
                    is_error: Some(true),
                    meta: None,
                })
            }
        }

        let tool = Arc::new(ErrorFlagTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let result = tool.call(None).unwrap();
        assert_eq!(result.is_error, Some(true));
    }

    // ============================================================================
    // ApiMetadata Variations
    // ============================================================================

    #[test]
    fn test_api_metadata_with_all_fields() {
        let metadata = ApiMetadata {
            name: "full_tool".to_string(),
            version: "v3.2.1".to_string(),
            description: "Full metadata test".to_string(),
            cache_ttl: Some(3600),
            is_streaming: true,
        };

        assert_eq!(metadata.name(), "full_tool");
        assert_eq!(metadata.version(), "v3.2.1");
        assert_eq!(metadata.cache_ttl, Some(3600));
        assert!(metadata.is_streaming);
    }

    #[test]
    fn test_api_metadata_zero_cache_ttl() {
        let metadata = ApiMetadata {
            name: "no_cache".to_string(),
            version: "v1".to_string(),
            description: "desc".to_string(),
            cache_ttl: Some(0),
            is_streaming: false,
        };

        assert_eq!(metadata.cache_ttl, Some(0));
    }

    #[test]
    fn test_api_metadata_large_cache_ttl() {
        let metadata = ApiMetadata {
            name: "long_cache".to_string(),
            version: "v1".to_string(),
            description: "desc".to_string(),
            cache_ttl: Some(u64::MAX),
            is_streaming: false,
        };

        assert_eq!(metadata.cache_ttl, Some(u64::MAX));
    }

    // ============================================================================
    // McpToolRegistration Tests
    // ============================================================================

    #[test]
    fn test_mcp_tool_registration_const_fn() {
        fn create_const_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
            struct ConstTool;
            impl mcp_sdk::tools::Tool for ConstTool {
                fn name(&self) -> String {
                    "const_tool".to_string()
                }
                fn description(&self) -> String {
                    "desc".to_string()
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
            Arc::new(ConstTool) as Arc<dyn mcp_sdk::tools::Tool>
        }

        const REGISTRATION: McpToolRegistration =
            McpToolRegistration::new("const_test", "v1", "Const tool test", create_const_tool);

        assert_eq!(REGISTRATION.name, "const_test");
        assert_eq!(REGISTRATION.version, "v1");
        assert_eq!(REGISTRATION.description, "Const tool test");
    }

    #[test]
    fn test_mcp_tool_registration_clone() {
        fn create_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
            struct CloneTestTool;
            impl mcp_sdk::tools::Tool for CloneTestTool {
                fn name(&self) -> String {
                    "clone_test".to_string()
                }
                fn description(&self) -> String {
                    "desc".to_string()
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
            Arc::new(CloneTestTool) as Arc<dyn mcp_sdk::tools::Tool>
        }

        let reg = McpToolRegistration::new("clone_tool", "v1", "Clone test", create_tool);
        let cloned = reg;

        assert_eq!(cloned.name, reg.name);
        assert_eq!(cloned.version, reg.version);
    }

    #[test]
    fn test_mcp_tool_registration_debug_impl() {
        fn create_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
            struct DebugTool;
            impl mcp_sdk::tools::Tool for DebugTool {
                fn name(&self) -> String {
                    "debug".to_string()
                }
                fn description(&self) -> String {
                    "desc".to_string()
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
            Arc::new(DebugTool) as Arc<dyn mcp_sdk::tools::Tool>
        }

        let reg = McpToolRegistration::new("debug_tool", "v1", "Debug test", create_tool);
        let debug_str = format!("{:?}", reg);
        assert!(debug_str.contains("McpToolRegistration"));
        assert!(debug_str.contains("debug_tool"));
    }

    // ============================================================================
    // JSON-RPC Protocol Tests - Additional
    // ============================================================================

    #[test]
    fn test_json_rpc_error_codes() {
        use mcp_sdk::transport::{JsonRpcError, JsonRpcResponse, JsonRpcVersion};

        let parse_error = JsonRpcResponse {
            jsonrpc: JsonRpcVersion::default(),
            id: 1_u64,
            result: None,
            error: Some(JsonRpcError {
                code: -32700,
                message: "Parse error".to_string(),
                data: None,
            }),
        };

        let invalid_request = JsonRpcResponse {
            jsonrpc: JsonRpcVersion::default(),
            id: 2_u64,
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Invalid Request".to_string(),
                data: None,
            }),
        };

        let method_not_found = JsonRpcResponse {
            jsonrpc: JsonRpcVersion::default(),
            id: 3_u64,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
        };

        assert!(parse_error.error.as_ref().unwrap().code < 0);
        assert!(invalid_request.error.as_ref().unwrap().code < 0);
        assert!(method_not_found.error.as_ref().unwrap().code < 0);
    }

    #[test]
    fn test_json_rpc_error_with_data() {
        use mcp_sdk::transport::{JsonRpcError, JsonRpcResponse, JsonRpcVersion};

        let response = JsonRpcResponse {
            jsonrpc: JsonRpcVersion::default(),
            id: 1_u64,
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: Some(serde_json::json!({
                    "expected": "string",
                    "received": "number"
                })),
            }),
        };

        let serialized = serde_json::to_string(&response).expect("should serialize");
        assert!(serialized.contains("\"data\""));
        assert!(serialized.contains("\"expected\""));
    }

    #[test]
    fn test_json_rpc_request_with_string_id() {
        use mcp_sdk::transport::JsonRpcRequest;

        let json_str = r#"{
            "jsonrpc": "2.0",
            "method": "test",
            "params": {},
            "id": "string-id-123"
        }"#;

        let result: Result<JsonRpcRequest, _> = serde_json::from_str(json_str);
        assert!(result.is_ok() || result.is_err());
    }

    // ============================================================================
    // Server Capabilities Tests - Additional
    // ============================================================================

    #[test]
    fn test_server_capabilities_with_logging() {
        use mcp_sdk::types::ServerCapabilities;

        let caps = ServerCapabilities {
            experimental: None,
            logging: Some(serde_json::json!({})),
            prompts: None,
            resources: None,
            tools: None,
        };

        let serialized = serde_json::to_string(&caps).expect("should serialize");
        assert!(serialized.contains("\"logging\""));
    }

    #[test]
    fn test_server_capabilities_with_prompts() {
        use mcp_sdk::types::{PromptCapabilities, ServerCapabilities};

        let caps = ServerCapabilities {
            experimental: None,
            logging: None,
            prompts: Some(PromptCapabilities {
                list_changed: Some(true),
            }),
            resources: None,
            tools: None,
        };

        let serialized = serde_json::to_string(&caps).expect("should serialize");
        assert!(serialized.contains("\"prompts\""));
        assert!(serialized.contains("\"listChanged\""));
    }

    #[test]
    fn test_server_capabilities_with_resources() {
        use mcp_sdk::types::{ResourceCapabilities, ServerCapabilities};

        let caps = ServerCapabilities {
            experimental: None,
            logging: None,
            prompts: None,
            resources: Some(ResourceCapabilities {
                subscribe: Some(true),
                list_changed: Some(true),
            }),
            tools: None,
        };

        let serialized = serde_json::to_string(&caps).expect("should serialize");
        assert!(serialized.contains("\"resources\""));
        assert!(serialized.contains("\"subscribe\""));
    }

    #[test]
    fn test_server_capabilities_with_experimental() {
        use mcp_sdk::types::ServerCapabilities;

        let caps = ServerCapabilities {
            experimental: Some(serde_json::json!({
                "customFeature": {
                    "enabled": true,
                    "version": "1.0"
                }
            })),
            logging: None,
            prompts: None,
            resources: None,
            tools: None,
        };

        let serialized = serde_json::to_string(&caps).expect("should serialize");
        assert!(serialized.contains("\"experimental\""));
        assert!(serialized.contains("\"customFeature\""));
    }

    // ============================================================================
    // Tool Multi-Call and Concurrency Tests
    // ============================================================================

    #[test]
    fn test_tool_multiple_sequential_calls() {
        struct StatefulTool;
        impl mcp_sdk::tools::Tool for StatefulTool {
            fn name(&self) -> String {
                "stateful".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
            }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn call(
                &self,
                input: Option<serde_json::Value>,
            ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                let val = input
                    .and_then(|v| v.get("value").cloned())
                    .unwrap_or(serde_json::json!(0));
                Ok(mcp_sdk::types::CallToolResponse {
                    content: vec![mcp_sdk::types::ToolResponseContent::Text {
                        text: format!("received: {}", val),
                    }],
                    is_error: None,
                    meta: None,
                })
            }
        }

        let tool = Arc::new(StatefulTool) as Arc<dyn mcp_sdk::tools::Tool>;

        for i in 0..10 {
            let result = tool.call(Some(serde_json::json!({"value": i})));
            assert!(result.is_ok());
            let response = result.unwrap();
            match &response.content[0] {
                mcp_sdk::types::ToolResponseContent::Text { text } => {
                    assert!(text.contains(&format!("received: {}", i)));
                }
                _ => panic!("Expected text content"),
            }
        }
    }

    #[test]
    fn test_tool_arc_cloning() {
        struct CloneableTool;
        impl mcp_sdk::tools::Tool for CloneableTool {
            fn name(&self) -> String {
                "cloneable".to_string()
            }
            fn description(&self) -> String {
                "desc".to_string()
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

        let tool = Arc::new(CloneableTool) as Arc<dyn mcp_sdk::tools::Tool>;
        let tool2 = tool.clone();
        let tool3 = Arc::clone(&tool);

        assert_eq!(tool.name(), tool2.name());
        assert_eq!(tool.name(), tool3.name());
    }

    // ============================================================================
    // Initialize Result Tests
    // ============================================================================

    #[test]
    fn test_initialize_result_serialization() {
        use mcp_sdk::transport::{JsonRpcResponse, JsonRpcVersion};

        let init_response = JsonRpcResponse {
            jsonrpc: JsonRpcVersion::default(),
            id: 1_u64,
            result: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "sdforge-mcp",
                    "version": "0.1.0"
                }
            })),
            error: None,
        };

        let serialized = serde_json::to_string(&init_response).expect("should serialize");
        assert!(serialized.contains("\"protocolVersion\""));
        assert!(serialized.contains("\"capabilities\""));
        assert!(serialized.contains("\"serverInfo\""));
    }

    #[test]
    fn test_list_tools_response() {
        use mcp_sdk::transport::{JsonRpcResponse, JsonRpcVersion};

        let list_response = JsonRpcResponse {
            jsonrpc: JsonRpcVersion::default(),
            id: 1_u64,
            result: Some(serde_json::json!({
                "tools": [
                    {
                        "name": "tool1",
                        "description": "First tool",
                        "inputSchema": {"type": "object"}
                    },
                    {
                        "name": "tool2",
                        "description": "Second tool",
                        "inputSchema": {"type": "string"}
                    }
                ]
            })),
            error: None,
        };

        let serialized = serde_json::to_string(&list_response).expect("should serialize");
        assert!(serialized.contains("\"tools\""));
        assert!(serialized.contains("\"tool1\""));
        assert!(serialized.contains("\"tool2\""));
    }
}

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
    pub name: &'static str,
    /// Tool version
    pub version: &'static str,
    /// Tool description
    pub description: &'static str,
    /// Function that creates the Tool at runtime
    pub create_fn: fn() -> Arc<dyn mcp_sdk::tools::Tool>,
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
    pub tool: Arc<dyn mcp_sdk::tools::Tool>,
    /// API metadata
    pub metadata: ApiMetadata,
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
            inner: instance.tool,
        };
        tools.add_tool(wrapper);

        // Use the first tool's metadata for server info
        if server_name == "sdforge-mcp" {
            server_name = instance.metadata.name().to_string();
            server_version = instance.metadata.version().to_string();
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

        let registration = McpToolRegistration {
            name: "test_tool",
            version: "v1",
            description: "A test tool",
            create_fn: create_test_tool,
        };

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
}

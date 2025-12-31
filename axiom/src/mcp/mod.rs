//! MCP server implementation

#[cfg(feature = "mcp")]
use serde_json::Value;
use crate::core::ApiMetadata;

/// MCP tool registration
#[derive(Clone)]
pub struct McpToolRegistration {
    /// Tool name
    pub name: &'static str,
    /// Tool description
    pub description: &'static str,
    /// Input schema JSON
    pub input_schema: Value,
    /// API metadata
    pub metadata: ApiMetadata,
}

#[cfg(feature = "mcp")]
inventory::collect!(McpToolRegistration);

/// Tool wrapper that implements mcp_sdk::tools::Tool trait
#[cfg(feature = "mcp")]
struct RegisteredTool {
    name: String,
    description: String,
    input_schema: Value,
    handler: fn(Option<Value>) -> Result<Value, String>,
}

#[cfg(feature = "mcp")]
impl mcp_sdk::tools::Tool for RegisteredTool {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn input_schema(&self) -> Value {
        self.input_schema.clone()
    }

    fn call(&self, input: Option<Value>) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
        let handler = self.handler;
        let result = handler(input);
        match result {
            Ok(value) => Ok(mcp_sdk::types::CallToolResponse {
                content: vec![mcp_sdk::types::ToolResponseContent::Text {
                    text: value.to_string(),
                }],
                is_error: Some(false),
                meta: None,
            }),
            Err(e) => Ok(mcp_sdk::types::CallToolResponse {
                content: vec![mcp_sdk::types::ToolResponseContent::Text {
                    text: e,
                }],
                is_error: Some(true),
                meta: None,
            }),
        }
    }
}

/// Build MCP server from registered tools
#[cfg(feature = "mcp")]
pub async fn build() -> mcp_sdk::server::Server<mcp_sdk::transport::ServerStdioTransport> {
    use mcp_sdk::server::Server;
    use mcp_sdk::tools::Tools;
    use mcp_sdk::transport::ServerStdioTransport;

    let mut tools = Tools::default();
    let mut server_name = "axiom-mcp".to_string();
    let mut server_version = "0.1.0".to_string();

    // Collect all registered tools
    for reg in inventory::iter::<McpToolRegistration> {
        // Create a simple wrapper that stores the registration data
        // The actual handler would need to be provided by the macro-generated code
        let tool = RegisteredTool {
            name: reg.name.to_string(),
            description: reg.description.to_string(),
            input_schema: reg.input_schema.clone(),
            handler: |_input: Option<Value>| Ok(serde_json::json!({"result": "ok"})),
        };
        tools.add_tool(tool);

        // Use the first registered tool's metadata for server info
        if server_name == "axiom-mcp" {
            server_name = reg.metadata.name.to_string();
            server_version = reg.metadata.version.to_string();
        }
    }

    Server::builder(ServerStdioTransport)
        .name(server_name)
        .version(server_version)
        .tools(tools)
        .build()
}

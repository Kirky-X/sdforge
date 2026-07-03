// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `SdForgeMcpServer` — MCP server that implements rmcp's `ServerHandler` trait.
//!
//! This module contains the server struct, its constructors, lookup methods,
//! and the `ServerHandler` trait implementation that bridges SDForge's tool
//! registry to rmcp's request/response model.

use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ErrorData, ListToolsResult, PaginatedRequestParams,
    ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::RoleServer;

use crate::mcp::get_mcp_tools;
use crate::mcp::handler::{value_to_json_object_arc, McpToolInstance};

/// MCP server that dispatches to tools registered via SDForge's inventory.
///
/// This struct implements `rmcp::handler::server::ServerHandler` so it can be
/// served via `ServiceExt::serve()` on any rmcp transport (stdio, HTTP, etc.).
/// It collects all `McpToolRegistration` entries at construction time and
/// dispatches `call_tool` requests to the matching tool.
#[derive(Clone)]
pub struct SdForgeMcpServer {
    /// Collected tool instances
    pub(crate) tools: Vec<McpToolInstance>,
    /// Server name (defaults to "sdforge-mcp")
    pub(crate) server_name: String,
    /// Server version (defaults to "0.2.0")
    pub(crate) server_version: String,
}

impl Default for SdForgeMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl SdForgeMcpServer {
    /// Create a new MCP server with all registered tools.
    ///
    /// Tools are collected from the `inventory` registry at construction time.
    /// The server name and version are derived from the first tool's metadata,
    /// or default to "sdforge-mcp" / "0.2.0" if no tools are registered.
    pub fn new() -> Self {
        Self::with_server_info("sdforge-mcp".to_string(), "0.2.0".to_string())
    }

    /// Create a server with explicit name/version but collect tools from inventory.
    pub fn with_server_info(server_name: String, server_version: String) -> Self {
        let tools = get_mcp_tools();
        let (name, version) = tools
            .first()
            .map(|t| {
                (
                    t.metadata().name().to_string(),
                    t.metadata().version().to_string(),
                )
            })
            .unwrap_or((server_name, server_version));
        Self {
            tools,
            server_name: name,
            server_version: version,
        }
    }

    /// Create a server with no tools (for testing or custom tool injection).
    pub fn empty() -> Self {
        Self {
            tools: Vec::new(),
            server_name: "sdforge-mcp".to_string(),
            server_version: "0.2.0".to_string(),
        }
    }

    /// Create a server with explicit tools (for testing or custom injection).
    pub fn with_tools(tools: Vec<McpToolInstance>) -> Self {
        let (name, version) = tools
            .first()
            .map(|t| {
                (
                    t.metadata().name().to_string(),
                    t.metadata().version().to_string(),
                )
            })
            .unwrap_or(("sdforge-mcp".to_string(), "0.2.0".to_string()));
        Self {
            tools,
            server_name: name,
            server_version: version,
        }
    }

    /// Get the number of registered tools.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Find a tool by name.
    pub fn find_tool(&self, name: &str) -> Option<&McpToolInstance> {
        self.tools.iter().find(|t| t.tool().name() == name)
    }

    /// Build a rmcp `Tool` model from a registered tool instance.
    pub(crate) fn build_tool_model(&self, instance: &McpToolInstance) -> Tool {
        let tool = instance.tool();
        // Convert serde_json::Value to Arc<JsonObject>.
        // If the schema is not an object, fall back to an empty object schema.
        let input_schema = value_to_json_object_arc(tool.input_schema());
        Tool {
            name: tool.name().to_string().into(),
            title: None,
            description: Some(tool.description().to_string().into()),
            input_schema,
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        }
    }

    /// Get all registered tools as rmcp `Tool` models (no RequestContext needed).
    ///
    /// This is the internal entry point used by `build_discovery_response` and
    /// tests. The public `ServerHandler::list_tools` delegates here.
    pub fn get_all_tools(&self) -> Vec<Tool> {
        self.tools
            .iter()
            .map(|instance| self.build_tool_model(instance))
            .collect()
    }

    /// Call a tool by name without a RequestContext (for testing and discovery).
    ///
    /// Returns the `CallToolResult` on success or `ErrorData` on failure.
    /// This is the internal entry point; the public `ServerHandler::call_tool`
    /// delegates here after extracting parameters from `CallToolRequestParams`.
    pub fn call_tool_internal(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let instance = self.find_tool(name).ok_or_else(|| {
            ErrorData::invalid_params(
                format!(
                    "Tool '{}' not found. Registered tools: {}",
                    name,
                    self.tools
                        .iter()
                        .map(|t| t.tool().name())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                None,
            )
        })?;
        instance.tool().call(arguments)
    }
}

impl ServerHandler for SdForgeMcpServer {
    fn get_info(&self) -> ServerInfo {
        use rmcp::model::{Implementation, ServerCapabilities};
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: self.server_name.clone(),
                title: None,
                version: self.server_version.clone(),
                description: None,
                icons: None,
                website_url: None,
            },
            instructions: None,
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let tools = self.get_all_tools();
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        // request.name is Cow<'static, str>; use deref via as_ref() to get &str.
        let name: &str = request.name.as_ref();
        // request.arguments is Option<JsonObject> (Map<String, Value>); convert to Value.
        let arguments = request.arguments.map(serde_json::Value::Object);
        self.call_tool_internal(name, arguments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{Extensions, Meta, NumberOrString};
    use rmcp::service::serve_directly;

    /// Dummy transport error type for test transport.
    #[derive(Debug)]
    struct DummyTransportError;

    impl std::fmt::Display for DummyTransportError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "dummy transport error")
        }
    }

    impl std::error::Error for DummyTransportError {}

    /// A no-op transport that immediately signals end-of-stream.
    ///
    /// Used with `serve_directly` to obtain a `Peer<RoleServer>` for
    /// constructing `RequestContext` in unit tests.
    struct DummyTransport;

    #[allow(clippy::manual_async_fn)] // trait requires `+ 'static`; async fn borrows self
    impl rmcp::transport::Transport<rmcp::RoleServer> for DummyTransport {
        type Error = DummyTransportError;

        fn send(
            &mut self,
            _item: rmcp::service::TxJsonRpcMessage<rmcp::RoleServer>,
        ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send + 'static {
            async { Ok(()) }
        }

        fn receive(
            &mut self,
        ) -> impl std::future::Future<
            Output = Option<rmcp::service::RxJsonRpcMessage<rmcp::RoleServer>>,
        > + Send {
            async { None }
        }

        fn close(&mut self) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
            async { Ok(()) }
        }
    }

    /// Build a `RequestContext<RoleServer>` for testing `ServerHandler` methods.
    ///
    /// Creates a `Peer` via `serve_directly` with a dummy transport, then
    /// constructs a `RequestContext` with default values for all fields except `peer`.
    fn make_test_context() -> RequestContext<RoleServer> {
        let server = SdForgeMcpServer::new();
        let running = serve_directly(server, DummyTransport, None);
        let peer = running.peer().clone();
        RequestContext {
            ct: Default::default(),
            id: NumberOrString::Number(0),
            meta: Meta::default(),
            extensions: Extensions::new(),
            peer,
        }
    }

    /// Test `ServerHandler::list_tools` returns all registered tools.
    ///
    /// Verifies that the trait method delegates to `get_all_tools()` and
    /// returns a `ListToolsResult` with the correct tool count and no cursor.
    #[tokio::test]
    async fn test_server_handler_list_tools_returns_all_tools() {
        let server = SdForgeMcpServer::new();
        let expected_count = server.tool_count();
        let context = make_test_context();

        let result = server.list_tools(None, context).await;
        assert!(result.is_ok(), "list_tools should succeed");

        let tools_result = result.unwrap();
        assert_eq!(
            tools_result.tools.len(),
            expected_count,
            "list_tools should return all registered tools"
        );
        assert!(
            tools_result.next_cursor.is_none(),
            "next_cursor should be None when all tools fit in one page"
        );
        assert!(
            tools_result.meta.is_none(),
            "meta should be None for a plain list_tools response"
        );

        // Verify each tool has a name and description
        for tool in &tools_result.tools {
            assert!(
                !tool.name.as_ref().is_empty(),
                "Each tool should have a non-empty name"
            );
            assert!(
                tool.description.is_some(),
                "Each tool should have a description"
            );
        }
    }

    /// Test `ServerHandler::list_tools` with pagination params.
    ///
    /// Verifies that passing `Some(PaginatedRequestParams)` with a cursor
    /// still returns all tools (pagination is not yet implemented).
    #[tokio::test]
    async fn test_server_handler_list_tools_with_pagination_params() {
        let server = SdForgeMcpServer::new();
        let context = make_test_context();

        let params = PaginatedRequestParams {
            meta: None,
            cursor: Some("cursor_abc".to_string()),
        };

        let result = server.list_tools(Some(params), context).await;
        assert!(
            result.is_ok(),
            "list_tools with pagination params should succeed"
        );

        let tools_result = result.unwrap();
        assert!(
            !tools_result.tools.is_empty(),
            "Should return tools even with a cursor"
        );
    }

    /// Test `ServerHandler::call_tool` with a valid tool name and no arguments.
    ///
    /// Verifies the name extraction (`request.name.as_ref()`) and the `None`
    /// arguments path (`arguments.map` returns `None`).
    #[tokio::test]
    async fn test_server_handler_call_tool_valid_no_args() {
        let server = SdForgeMcpServer::new();
        let context = make_test_context();

        let request = CallToolRequestParams {
            meta: None,
            name: "coverage_test_tool".into(),
            arguments: None,
            task: None,
        };

        let result = server.call_tool(request, context).await;
        assert!(result.is_ok(), "call_tool should succeed for valid tool");

        let tool_result = result.unwrap();
        // coverage_test_tool returns empty content
        assert!(
            tool_result.content.is_empty(),
            "coverage_test_tool should return empty content"
        );
        assert!(
            tool_result.is_error.is_none(),
            "is_error should be None for successful call"
        );
    }

    /// Test `ServerHandler::call_tool` with a valid tool name and JSON arguments.
    ///
    /// Verifies the `Some(arguments)` path: `request.arguments.map(serde_json::Value::Object)`
    /// converts the `JsonObject` to a `serde_json::Value::Object`.
    #[tokio::test]
    async fn test_server_handler_call_tool_valid_with_args() {
        let server = SdForgeMcpServer::new();
        let context = make_test_context();

        let mut args = serde_json::Map::new();
        args.insert(
            "param1".to_string(),
            serde_json::Value::String("value1".to_string()),
        );
        args.insert(
            "param2".to_string(),
            serde_json::Value::Number(serde_json::Number::from(42)),
        );

        let request = CallToolRequestParams {
            meta: None,
            name: "coverage_test_tool".into(),
            arguments: Some(args),
            task: None,
        };

        let result = server.call_tool(request, context).await;
        assert!(
            result.is_ok(),
            "call_tool with arguments should succeed for coverage_test_tool"
        );
    }

    /// Test `ServerHandler::call_tool` with a nonexistent tool name.
    ///
    /// Verifies the error path: `call_tool_internal` returns `ErrorData::invalid_params`
    /// with a message listing registered tools.
    #[tokio::test]
    async fn test_server_handler_call_tool_nonexistent() {
        let server = SdForgeMcpServer::new();
        let context = make_test_context();

        let request = CallToolRequestParams {
            meta: None,
            name: "does_not_exist".into(),
            arguments: None,
            task: None,
        };

        let result = server.call_tool(request, context).await;
        assert!(
            result.is_err(),
            "call_tool with nonexistent tool should return error"
        );

        let error = result.unwrap_err();
        assert!(
            error.message.contains("not found"),
            "Error message should contain 'not found', got: {}",
            error.message
        );
        // The error message should list registered tools
        assert!(
            error.message.contains("coverage_test_tool"),
            "Error message should list registered tools, got: {}",
            error.message
        );
    }

    /// Test `ServerHandler::call_tool` with an empty tool name.
    ///
    /// Verifies that an empty name is handled gracefully (returns error).
    #[tokio::test]
    async fn test_server_handler_call_tool_empty_name() {
        let server = SdForgeMcpServer::new();
        let context = make_test_context();

        let request = CallToolRequestParams {
            meta: None,
            name: "".into(),
            arguments: None,
            task: None,
        };

        let result = server.call_tool(request, context).await;
        assert!(
            result.is_err(),
            "call_tool with empty name should return error"
        );
    }

    /// Test `ServerHandler::call_tool` with task metadata.
    ///
    /// Verifies that the `task` field in `CallToolRequestParams` is accepted
    /// (even though the current implementation doesn't use it).
    #[tokio::test]
    async fn test_server_handler_call_tool_with_task_metadata() {
        let server = SdForgeMcpServer::new();
        let context = make_test_context();

        let mut task_meta = serde_json::Map::new();
        task_meta.insert(
            "task_id".to_string(),
            serde_json::Value::String("task_123".to_string()),
        );

        let request = CallToolRequestParams {
            meta: None,
            name: "coverage_test_tool".into(),
            arguments: None,
            task: Some(task_meta),
        };

        let result = server.call_tool(request, context).await;
        assert!(
            result.is_ok(),
            "call_tool with task metadata should succeed for coverage_test_tool"
        );
    }
}

// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `SdForgeMcpServer` — MCP server that implements rmcp's `ServerHandler` trait.
//!
//! This module contains the server struct, its constructors, lookup methods,
//! and the `ServerHandler` trait implementation that bridges SDForge's tool
//! registry to rmcp's request/response model.

use rmcp::RoleServer;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ErrorData, ListToolsResult, PaginatedRequestParams,
    ServerInfo, Tool,
};
use rmcp::service::RequestContext;

use crate::mcp::McpToolInstance;
use crate::mcp::get_mcp_tools;
use crate::mcp::value_to_json_object_arc;

/// Maximum allowed size (in bytes) for `call_tool` arguments payloads.
///
/// Defends against memory-exhaustion attacks where a client submits a huge
/// JSON blob to `tools/call`. The limit is applied to the serialized JSON
/// length before the arguments reach the tool implementation.
///
/// 1 MiB matches the typical MCP server default and leaves ample headroom
/// for legitimate tool inputs (most tool calls are < 4 KiB).
pub const MAX_ARGUMENTS_SIZE_BYTES: usize = 0x10_0000;

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
        let mut model = Tool::default();
        model.name = tool.name().to_string().into();
        model.description = Some(tool.description().to_string().into());
        model.input_schema = input_schema;
        model
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
    ///
    /// # Security
    ///
    /// - Arguments payload size is capped at [`MAX_ARGUMENTS_SIZE_BYTES`].
    ///   Larger payloads are rejected with `invalid_params` before reaching
    ///   the tool implementation (DoS defense).
    /// - Unknown tool names return a generic `"tool not found"` error without
    ///   listing registered tools (information disclosure defense).
    pub fn call_tool_internal(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        // vuln-0002: reject oversized argument payloads before any dispatch.
        if let Some(ref args) = arguments {
            let size = serde_json::to_vec(args).map(|v| v.len()).unwrap_or(0);
            if size > MAX_ARGUMENTS_SIZE_BYTES {
                return Err(ErrorData::invalid_params(
                    format!(
                        "arguments payload size ({}) exceeds maximum allowed size ({})",
                        size, MAX_ARGUMENTS_SIZE_BYTES
                    ),
                    None,
                ));
            }
        }

        // vuln-0002: do not leak the registered tool list in the error
        // message — a generic "tool not found" is sufficient for clients
        // while denying an attacker an inventory enumeration vector.
        let instance = self
            .find_tool(name)
            .ok_or_else(|| ErrorData::invalid_params(format!("tool not found: {}", name), None))?;

        // vuln-0002: validate arguments against the tool's input_schema before
        // dispatch. Closes the gap where hand-written `SdForgeTool` impls that
        // forgot to validate arguments in `call()` would accept arbitrary input.
        // Macros-generated tools already enforce `#[serde(deny_unknown_fields)]`,
        // but this entry-point check defends against hand-written tools and
        // provides defense-in-depth for all code paths.
        if let Some(ref args) = arguments {
            let schema = instance.tool().input_schema();
            super::schema_validation::validate_arguments(&schema, args)?;
        }

        instance.tool().call(arguments)
    }
}

impl ServerHandler for SdForgeMcpServer {
    fn get_info(&self) -> ServerInfo {
        use rmcp::model::{Implementation, ServerCapabilities};
        let mut server_info = Implementation::default();
        server_info.name = self.server_name.clone();
        server_info.version = self.server_version.clone();
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(server_info)
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
    use rmcp::model::{NumberOrString, TaskMetadata};
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
        RequestContext::new(NumberOrString::Number(0), peer)
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

        let mut params = PaginatedRequestParams::default();
        params.cursor = Some("cursor_abc".to_string());

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

        let request = CallToolRequestParams::new("coverage_test_tool");

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

        let mut request = CallToolRequestParams::new("coverage_test_tool");
        request.arguments = Some(args);

        let result = server.call_tool(request, context).await;
        assert!(
            result.is_ok(),
            "call_tool with arguments should succeed for coverage_test_tool"
        );
    }

    /// Test `ServerHandler::call_tool` with a nonexistent tool name.
    ///
    /// Verifies the error path: `call_tool_internal` returns `ErrorData::invalid_params`
    /// with a generic "tool not found" message. The message must NOT leak the
    /// registered tool list (vuln-0002: information disclosure defense).
    #[tokio::test]
    async fn test_server_handler_call_tool_nonexistent() {
        let server = SdForgeMcpServer::new();
        let context = make_test_context();

        let request = CallToolRequestParams::new("does_not_exist");

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
        // vuln-0002: error message MUST NOT enumerate registered tools.
        assert!(
            !error.message.contains("coverage_test_tool"),
            "Error message must not leak registered tool names, got: {}",
            error.message
        );
        assert!(
            !error.message.contains("Registered tools"),
            "Error message must not contain 'Registered tools' list, got: {}",
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

        let request = CallToolRequestParams::new("");

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

        let request = {
            let mut params = CallToolRequestParams::new("coverage_test_tool");
            params.task = Some(TaskMetadata::new());
            params
        };

        let result = server.call_tool(request, context).await;
        assert!(
            result.is_ok(),
            "call_tool with task metadata should succeed for coverage_test_tool"
        );
    }

    // ========================================================================
    // vuln-0002 security tests: argument size limit + non-disclosure
    // ========================================================================

    /// Verify `call_tool_internal` rejects argument payloads larger than
    /// `MAX_ARGUMENTS_SIZE_BYTES` (1 MiB) with `invalid_params`.
    ///
    /// A 2 MiB string value must be rejected before reaching the tool.
    #[test]
    fn test_call_tool_internal_rejects_oversized_arguments() {
        let server = SdForgeMcpServer::new();
        // Build a payload well over the 1 MiB limit.
        let oversized = serde_json::Value::String("x".repeat(2 * 1024 * 1024));
        let result = server.call_tool_internal("coverage_test_tool", Some(oversized));
        assert!(
            result.is_err(),
            "oversized arguments payload must be rejected"
        );
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("exceeds maximum") || msg.contains("payload size"),
            "error should mention size limit, got: {msg}"
        );
    }

    /// Verify `call_tool_internal` accepts payloads just under the size limit.
    #[test]
    fn test_call_tool_internal_accepts_payload_under_limit() {
        let server = SdForgeMcpServer::new();
        // 512 KiB object — well under the 1 MiB limit, and matches the
        // coverage_test_tool's input_schema (`{"type": "object"}`).
        let ok_payload = serde_json::json!({"data": "x".repeat(512 * 1024)});
        let result = server.call_tool_internal("coverage_test_tool", Some(ok_payload));
        assert!(
            result.is_ok(),
            "payload under the size limit should be accepted"
        );
    }

    /// Verify `call_tool_internal` returns a generic `tool not found` error
    /// and does NOT enumerate registered tool names (vuln-0002 fix).
    #[test]
    fn test_call_tool_internal_does_not_leak_tool_list() {
        let server = SdForgeMcpServer::new();
        let result = server.call_tool_internal("nonexistent_tool_xyz", None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("not found"),
            "error should mention 'not found', got: {msg}"
        );
        // The error must not leak the registered tool inventory.
        assert!(
            !msg.contains("coverage_test_tool"),
            "error must not leak registered tool names, got: {msg}"
        );
        assert!(
            !msg.contains("Registered tools"),
            "error must not contain 'Registered tools' phrase, got: {msg}"
        );
    }

    /// Verify `MAX_ARGUMENTS_SIZE_BYTES` is exactly 1 MiB (documented contract).
    #[test]
    fn test_max_arguments_size_bytes_is_one_mib() {
        assert_eq!(MAX_ARGUMENTS_SIZE_BYTES, 1024 * 1024);
    }
}

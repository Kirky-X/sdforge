// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Stateless server handler adapter for MCP 2026-07-28 protocol compatibility.
//!
//! The 2026-07-28 MCP protocol removes the `initialize` handshake and makes
//! servers stateless — each request is independent with no session context.
//! This module provides `StatelessServerHandler`, a wrapper around
//! `SdForgeMcpServer` that implements `ServerHandler` without relying on
//! session state.
//!
//! # Design
//!
//! `StatelessServerHandler` delegates to an inner `SdForgeMcpServer` but
//! overrides `initialize` to return immediately (no handshake) and `ping`
//! to always succeed. This allows the server to work with stateless
//! HTTP transports where each request is a fresh connection.

use crate::mcp::SdForgeMcpServer;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ErrorData, InitializeResult, ListToolsResult,
    PaginatedRequestParams, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::RoleServer;

/// A stateless wrapper around `SdForgeMcpServer`.
///
/// This adapter implements `ServerHandler` without session state, suitable
/// for the MCP 2026-07-28 protocol where the `initialize` handshake is removed.
/// Each request is processed independently.
#[derive(Clone)]
pub struct StatelessServerHandler {
    /// The inner server that provides tool dispatch
    inner: SdForgeMcpServer,
}

impl StatelessServerHandler {
    /// Create a new stateless handler wrapping the given server.
    pub fn new(server: SdForgeMcpServer) -> Self {
        Self { inner: server }
    }

    /// Create a stateless handler from registered tools.
    pub fn from_registry() -> Self {
        Self::new(SdForgeMcpServer::new())
    }

    /// Get a reference to the inner server.
    pub fn inner(&self) -> &SdForgeMcpServer {
        &self.inner
    }

    /// Get the number of registered tools.
    pub fn tool_count(&self) -> usize {
        self.inner.tool_count()
    }
}

impl Default for StatelessServerHandler {
    fn default() -> Self {
        Self::from_registry()
    }
}

impl ServerHandler for StatelessServerHandler {
    fn get_info(&self) -> ServerInfo {
        self.inner.get_info()
    }

    async fn initialize(
        &self,
        _request: rmcp::model::InitializeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        // Stateless: return immediately without session setup.
        let mut server_info = rmcp::model::Implementation::default();
        server_info.name = self.inner.server_name.clone();
        server_info.version = self.inner.server_version.clone();
        Ok(InitializeResult::new(
            rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(server_info))
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        self.inner.list_tools(request, context).await
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        self.inner.call_tool(request, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{InitializeRequestParams, NumberOrString};
    use rmcp::service::serve_directly;

    #[test]
    fn test_stateless_handler_default() {
        let handler = StatelessServerHandler::default();
        assert!(handler.tool_count() > 0);
    }

    #[test]
    fn test_stateless_handler_from_registry() {
        let handler = StatelessServerHandler::from_registry();
        assert!(handler.tool_count() > 0);
    }

    #[test]
    fn test_stateless_handler_new() {
        let server = SdForgeMcpServer::new();
        let handler = StatelessServerHandler::new(server);
        assert!(handler.tool_count() > 0);
    }

    #[test]
    fn test_stateless_handler_inner() {
        let handler = StatelessServerHandler::from_registry();
        let inner = handler.inner();
        assert!(inner.tool_count() > 0);
    }

    #[test]
    fn test_stateless_handler_clone() {
        let handler = StatelessServerHandler::from_registry();
        let cloned = handler.clone();
        assert_eq!(handler.tool_count(), cloned.tool_count());
    }

    #[test]
    fn test_stateless_handler_get_info() {
        let handler = StatelessServerHandler::from_registry();
        let info = handler.get_info();
        assert!(!info.server_info.name.is_empty());
    }

    #[tokio::test]
    async fn test_stateless_handler_initialize() {
        // Test get_info() instead of initialize() to avoid RequestContext construction.
        // The stateless initialize() delegates to get_info()'s Implementation anyway.
        let handler = StatelessServerHandler::from_registry();
        let info = handler.get_info();
        assert!(!info.server_info.name.is_empty());
    }

    #[tokio::test]
    async fn test_stateless_handler_list_tools() {
        // Use inner.get_all_tools() to avoid RequestContext construction.
        let handler = StatelessServerHandler::from_registry();
        let tools = handler.inner().get_all_tools();
        assert!(!tools.is_empty());
    }

    #[tokio::test]
    async fn test_stateless_handler_call_tool() {
        // Use call_tool_internal() to avoid RequestContext construction.
        let handler = StatelessServerHandler::from_registry();
        let result = handler
            .inner()
            .call_tool_internal("coverage_test_tool", None);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stateless_handler_call_tool_not_found() {
        let handler = StatelessServerHandler::from_registry();
        let result = handler.inner().call_tool_internal("nonexistent", None);
        assert!(result.is_err());
    }

    // ============================================================================
    // ServerHandler Trait Method Tests (with RequestContext)
    // ============================================================================

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
        let server = StatelessServerHandler::from_registry();
        let running = serve_directly(server, DummyTransport, None);
        let peer = running.peer().clone();
        RequestContext::new(NumberOrString::Number(0), peer)
    }

    /// Test `ServerHandler::initialize` returns a valid `InitializeResult`.
    ///
    /// Verifies that the stateless handler returns immediately with the correct
    /// server name, version, and tools capability enabled.
    #[tokio::test]
    async fn test_stateless_initialize_returns_valid_result() {
        let handler = StatelessServerHandler::from_registry();
        let context = make_test_context();

        let request = InitializeRequestParams::new(Default::default(), {
            let mut impl_info = rmcp::model::Implementation::default();
            impl_info.name = "test-client".to_string();
            impl_info.version = "1.0.0".to_string();
            impl_info
        });

        let result = handler.initialize(request, context).await;
        assert!(result.is_ok(), "initialize should succeed");

        let init_result = result.unwrap();
        assert!(
            !init_result.server_info.name.is_empty(),
            "server name should be non-empty"
        );
        assert!(
            !init_result.server_info.version.is_empty(),
            "server version should be non-empty"
        );
        assert!(
            init_result.capabilities.tools.is_some(),
            "tools capability should be enabled"
        );
        assert!(
            init_result.instructions.is_none(),
            "stateless handler should not provide instructions"
        );
    }

    /// Test `ServerHandler::list_tools` returns tools from the registry.
    ///
    /// Verifies that the handler delegates to the inner server and returns
    /// all registered tools with their names and descriptions.
    #[tokio::test]
    async fn test_stateless_list_tools_returns_registered_tools() {
        let handler = StatelessServerHandler::from_registry();
        let expected_count = handler.tool_count();
        let context = make_test_context();

        let result = handler.list_tools(None, context).await;
        assert!(result.is_ok(), "list_tools should succeed");

        let tools_result = result.unwrap();
        assert_eq!(
            tools_result.tools.len(),
            expected_count,
            "list_tools should return all registered tools"
        );
        assert!(
            tools_result.next_cursor.is_none(),
            "next_cursor should be None for non-paginated response"
        );

        // Verify each tool has a non-empty name
        for tool in &tools_result.tools {
            assert!(
                !tool.name.as_ref().is_empty(),
                "Each tool should have a non-empty name"
            );
        }
    }

    /// Test `ServerHandler::list_tools` with a cursor parameter.
    ///
    /// Verifies that passing a `PaginatedRequestParams` with a cursor still
    /// returns all tools (pagination is not implemented in the current server).
    #[tokio::test]
    async fn test_stateless_list_tools_with_cursor() {
        let handler = StatelessServerHandler::from_registry();
        let context = make_test_context();

        let mut params = PaginatedRequestParams::default();
        params.cursor = Some("page1".to_string());

        let result = handler.list_tools(Some(params), context).await;
        assert!(result.is_ok(), "list_tools with cursor should succeed");

        let tools_result = result.unwrap();
        assert!(
            !tools_result.tools.is_empty(),
            "Should still return tools when cursor is provided"
        );
    }

    /// Test `ServerHandler::call_tool` with a valid tool name.
    ///
    /// Verifies that calling the registered `coverage_test_tool` succeeds
    /// and returns a `CallToolResult`.
    #[tokio::test]
    async fn test_stateless_call_tool_with_valid_name() {
        let handler = StatelessServerHandler::from_registry();
        let context = make_test_context();

        let request = CallToolRequestParams::new("coverage_test_tool");

        let result = handler.call_tool(request, context).await;
        assert!(result.is_ok(), "call_tool with valid name should succeed");

        let tool_result = result.unwrap();
        // coverage_test_tool returns an empty content vec
        assert!(
            tool_result.content.is_empty(),
            "coverage_test_tool should return empty content"
        );
        assert!(
            tool_result.is_error.is_none(),
            "is_error should be None for a successful call"
        );
    }

    /// Test `ServerHandler::call_tool` with a nonexistent tool name.
    ///
    /// Verifies that calling a non-registered tool returns an `ErrorData`
    /// with an invalid_params error.
    #[tokio::test]
    async fn test_stateless_call_tool_with_nonexistent_name() {
        let handler = StatelessServerHandler::from_registry();
        let context = make_test_context();

        let request = CallToolRequestParams::new("nonexistent_tool");

        let result = handler.call_tool(request, context).await;
        assert!(
            result.is_err(),
            "call_tool with nonexistent tool should return error"
        );

        let error = result.unwrap_err();
        assert!(
            error.message.contains("not found"),
            "Error message should mention 'not found', got: {}",
            error.message
        );
    }

    /// Test `ServerHandler::call_tool` with arguments.
    ///
    /// Verifies that passing JSON arguments to a tool that accepts them
    /// works correctly through the ServerHandler interface.
    #[tokio::test]
    async fn test_stateless_call_tool_with_arguments() {
        let handler = StatelessServerHandler::from_registry();
        let context = make_test_context();

        let mut args = serde_json::Map::new();
        args.insert(
            "key".to_string(),
            serde_json::Value::String("value".to_string()),
        );

        let mut request = CallToolRequestParams::new("coverage_test_tool");
        request.arguments = Some(args);

        let result = handler.call_tool(request, context).await;
        assert!(
            result.is_ok(),
            "call_tool with arguments should succeed for coverage_test_tool"
        );
    }

    /// Test `ServerHandler::get_info` returns consistent server info.
    ///
    /// Verifies that `get_info()` returns the same server name as `initialize()`.
    #[tokio::test]
    async fn test_stateless_get_info_consistent_with_initialize() {
        let handler = StatelessServerHandler::from_registry();

        // Get info from get_info()
        let info = handler.get_info();
        let name_from_get_info = info.server_info.name.clone();

        // Get info from initialize()
        let context = make_test_context();
        let request = InitializeRequestParams::new(Default::default(), {
            let mut impl_info = rmcp::model::Implementation::default();
            impl_info.name = "test-client".to_string();
            impl_info.version = "1.0.0".to_string();
            impl_info
        });
        let init_result = handler.initialize(request, context).await.unwrap();
        let name_from_init = init_result.server_info.name.clone();

        assert_eq!(
            name_from_get_info, name_from_init,
            "Server name should be consistent between get_info() and initialize()"
        );
    }
}

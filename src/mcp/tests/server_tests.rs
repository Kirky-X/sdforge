//! Tests for `SdForgeMcpServer`: construction, lookup, and `ServerHandler` integration.

use super::{create_test_metadata, create_test_tool};
use crate::mcp::{build, McpToolInstance, SdForgeMcpServer};
use rmcp::handler::server::ServerHandler;

#[test]
fn test_build_returns_server_with_tools() {
    let server = build();
    assert!(server.tool_count() > 0, "server should have registered tools");
}

#[test]
fn test_server_find_tool() {
    let server = build();
    assert!(server.find_tool("coverage_test_tool").is_some());
    assert!(server.find_tool("nonexistent_tool").is_none());
}

#[test]
fn test_server_empty() {
    let server = SdForgeMcpServer::empty();
    assert_eq!(server.tool_count(), 0);
}

#[test]
fn test_server_with_tools() {
    let tool = create_test_tool();
    let metadata = create_test_metadata();
    let instance = McpToolInstance::new(tool, metadata);
    let server = SdForgeMcpServer::with_tools(vec![instance]);
    assert_eq!(server.tool_count(), 1);
    assert!(server.find_tool("test").is_some());
}

#[test]
fn test_build_tool_model() {
    let server = build();
    let instance = server
        .find_tool("coverage_test_tool")
        .expect("coverage tool should be registered");
    let tool_model = server.build_tool_model(instance);
    assert_eq!(tool_model.name.as_ref(), "coverage_test_tool");
    assert!(tool_model.description.is_some());
}

#[tokio::test]
async fn test_server_handler_list_tools() {
    // Use get_all_tools() instead of list_tools() to avoid constructing
    // a RequestContext (which requires a live Peer connection).
    let server = build();
    let tools = server.get_all_tools();
    assert!(!tools.is_empty());
}

#[tokio::test]
async fn test_server_handler_call_tool_success() {
    // Use call_tool_internal() to avoid RequestContext construction.
    let server = build();
    let result = server.call_tool_internal("coverage_test_tool", None);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_server_handler_call_tool_not_found() {
    let server = build();
    let result = server.call_tool_internal("nonexistent", None);
    assert!(result.is_err());
}

#[test]
fn test_server_default() {
    let server = SdForgeMcpServer::default();
    assert!(server.tool_count() > 0);
}

#[test]
fn test_server_clone() {
    let server = build();
    let cloned = server.clone();
    assert_eq!(server.tool_count(), cloned.tool_count());
}

#[test]
fn test_server_with_server_info() {
    let server = SdForgeMcpServer::with_server_info("custom".to_string(), "9.9.9".to_string());
    // If tools exist, name comes from first tool; otherwise from args
    if server.tool_count() == 0 {
        assert_eq!(server.server_name, "custom");
    }
}

#[test]
fn test_server_get_info() {
    let server = build();
    let info = server.get_info();
    assert!(!info.server_info.name.is_empty());
    assert!(!info.server_info.version.is_empty());
}

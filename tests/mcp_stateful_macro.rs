// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Macro-generated stateful MCP tools (plan §5.3): a `#[forge(tool_name = ...)]`
//! endpoint that declares `#[state]` compiles into a real `call_with_ctx`
//! dispatch — registered via inventory, discoverable by name, and executable
//! once the server is built via `SdForgeMcpServer::with_state(...)`.
//!
//! Compat: in 0.5.1 this same endpoint generated an always-error stub; the
//! new fail-explicit guidance error replaces it only on the unconfigured path.

#![cfg(feature = "mcp")]

use std::sync::Arc;

/// Shared application state recovered by the macro-generated tool.
#[derive(Debug)]
pub struct AppState {
    pub tag: &'static str,
}

#[sdforge::forge(
    name = "mcp_stateful_echo",
    version = "v1",
    description = "stateful MCP echo",
    path = "/mcp-stateful/echo",
    method = "POST",
    tool_name = "macro_stateful_echo"
)]
async fn mcp_stateful_echo(
    #[state] state: Arc<AppState>,
    msg: String,
) -> Result<serde_json::Value, sdforge::core::ApiError> {
    Ok(serde_json::json!({ "echo": msg, "state": state.tag }))
}

/// `#[state]` + `tool_name` endpoints auto-register via inventory — in 0.5.1
/// they also registered, but always errored at dispatch time.
#[test]
fn macro_stateful_tool_is_registered() {
    let server = sdforge::mcp::SdForgeMcpServer::new();
    assert!(
        server.find_tool("macro_stateful_echo").is_some(),
        "#[state] + tool_name endpoint must auto-register as an MCP tool"
    );
}

/// `with_state(...)` → the generated `call_with_ctx` dispatches for real and
/// the tool sees both the deserialized params and the state.
#[test]
fn macro_stateful_dispatch_with_state_succeeds() {
    let server = sdforge::mcp::SdForgeMcpServer::new()
        .with_state(Arc::new(AppState { tag: "macro" }) as Arc<dyn std::any::Any + Send + Sync>);
    let result = server
        .call_tool_internal(
            "macro_stateful_echo",
            Some(serde_json::json!({ "msg": "hello" })),
        )
        .expect("dispatch must succeed with state injected");
    assert_ne!(result.is_error, Some(true));
    let text = match &result.content[0] {
        sdforge::rmcp::model::ContentBlock::Text(t) => t.text.clone(),
        other => panic!("expected text content, got {other:?}"),
    };
    assert!(
        text.contains("hello"),
        "params must reach the handler: {text}"
    );
    assert!(
        text.contains("macro"),
        "state must reach the handler: {text}"
    );
}

/// Without `with_state(...)` → fail-explicit `forge.state_missing` (rendered
/// as the handler `ApiError` JSON, carrying the machine-readable error_id).
#[test]
fn macro_stateful_dispatch_without_state_fails_explicit() {
    let server = sdforge::mcp::SdForgeMcpServer::new();
    let result = server
        .call_tool_internal(
            "macro_stateful_echo",
            Some(serde_json::json!({ "msg": "hi" })),
        )
        .expect("state-missing surfaces as a CallToolResult error, not an Err");
    assert_eq!(result.is_error, Some(true), "must be a tool-level error");
    let text = match &result.content[0] {
        sdforge::rmcp::model::ContentBlock::Text(t) => t.text.clone(),
        other => panic!("expected text content, got {other:?}"),
    };
    assert!(
        text.contains("forge.state_missing"),
        "serialized ApiError must carry error_id, got: {text}"
    );
}

/// Type mismatch → fail-explicit `forge.state_type_mismatch`.
#[test]
fn macro_stateful_dispatch_with_wrong_state_type_fails_explicit() {
    let server = sdforge::mcp::SdForgeMcpServer::new()
        .with_state(Arc::new(42_u32) as Arc<dyn std::any::Any + Send + Sync>);
    let result = server
        .call_tool_internal(
            "macro_stateful_echo",
            Some(serde_json::json!({ "msg": "hi" })),
        )
        .expect("type-mismatch surfaces as a CallToolResult error, not an Err");
    assert_eq!(result.is_error, Some(true), "must be a tool-level error");
    let text = match &result.content[0] {
        sdforge::rmcp::model::ContentBlock::Text(t) => t.text.clone(),
        other => panic!("expected text content, got {other:?}"),
    };
    assert!(
        text.contains("forge.state_type_mismatch"),
        "serialized ApiError must carry error_id, got: {text}"
    );
}

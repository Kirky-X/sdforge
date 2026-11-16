// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `ToolCallContext` wiring tests — application state reaches tools via
//! `call_with_ctx` (plan §5.1).
//!
//! Covers the three state outcomes of a hand-written tool that overrides
//! `SdForgeTool::call_with_ctx`:
//!
//! 1. `with_state(...)` → the tool recovers the concrete `Arc<T>` through
//!    `downcast_state(ctx.state())`;
//! 2. without `with_state` → fail-explicit `forge.state_missing`;
//! 3. wrong state type → fail-explicit `forge.state_type_mismatch`.
//!
//! The default-delegation compat path (tools that only implement `call`)
//! is exercised implicitly by every existing server/handler test.

#![cfg(feature = "mcp")]

use super::create_test_metadata;
use crate::core::downcast_state;
use crate::mcp::{McpToolInstance, SdForgeMcpServer, SdForgeTool, ToolCallContext};
use rmcp::model::{CallToolResult, ContentBlock, ErrorCode, ErrorData as McpError};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Marker state type (unique here so the mismatch case is deterministic).
#[derive(Debug)]
struct ProbeState {
    value: &'static str,
}

/// Tool that overrides `call_with_ctx` and records what the context carried.
struct ProbeTool {
    seen_state: Mutex<Vec<String>>,
}

impl ProbeTool {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            seen_state: Mutex::new(Vec::new()),
        })
    }
}

impl SdForgeTool for ProbeTool {
    fn name(&self) -> &str {
        "probe_tool"
    }

    fn description(&self) -> &str {
        "records what ToolCallContext carried"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({"type": "object"})
    }

    // Fail-explicit: a probe tool is only meaningful through `call_with_ctx`.
    fn call(&self, _input: Option<Value>) -> Result<CallToolResult, McpError> {
        Err(McpError::internal_error(
            "probe tool must be dispatched via call_with_ctx",
            None,
        ))
    }

    fn call_with_ctx(
        &self,
        _input: Option<Value>,
        ctx: &ToolCallContext,
    ) -> Result<CallToolResult, McpError> {
        let probe: Arc<ProbeState> = downcast_state::<ProbeState>(ctx.state())
            .map_err(|e| McpError::internal_error(format!("{e:?}"), None))?;
        self.seen_state
            .lock()
            .expect("mutex not poisoned")
            .push(probe.value.to_string());
        Ok(CallToolResult::success(vec![ContentBlock::text("probed")]))
    }
}

fn probe_server(state: crate::core::HandlerState) -> (SdForgeMcpServer, Arc<ProbeTool>) {
    let tool = ProbeTool::new();
    let mut server = SdForgeMcpServer::with_tools(vec![McpToolInstance::new(
        tool.clone(),
        create_test_metadata(),
    )]);
    if let Some(state) = state {
        server = server.with_state(state);
    }
    (server, tool)
}

/// Outcome 1: state injected → tool receives the concrete `Arc<ProbeState>`.
#[test]
fn with_state_reaches_tool_via_call_with_ctx() {
    let (server, tool) = probe_server(Some(Arc::new(ProbeState { value: "injected" })));
    let result = server
        .call_tool_internal("probe_tool", None)
        .expect("dispatch must succeed with state injected");
    assert_ne!(result.is_error, Some(true));
    assert_eq!(
        *tool.seen_state.lock().expect("mutex not poisoned"),
        vec!["injected".to_string()],
        "tool must observe the injected state"
    );
}

/// Outcome 2: no state injected → fail-explicit `forge.state_missing`.
#[test]
fn without_with_state_dispatch_fails_explicit() {
    let (server, tool) = probe_server(None);
    let err = server
        .call_tool_internal("probe_tool", None)
        .expect_err("missing state must fail explicitly");
    assert_eq!(err.code, ErrorCode(-32603), "internal_error");
    assert!(
        err.message.contains("forge.state_missing"),
        "error must carry the machine-readable error_id, got: {}",
        err.message
    );
    assert!(
        tool.seen_state
            .lock()
            .expect("mutex not poisoned")
            .is_empty(),
        "tool must not run without state"
    );
}

/// Outcome 3: wrong state type → fail-explicit `forge.state_type_mismatch`.
#[test]
fn state_type_mismatch_fails_explicit() {
    let (server, tool) = probe_server(Some(Arc::new(42_i32)));
    let err = server
        .call_tool_internal("probe_tool", None)
        .expect_err("type mismatch must fail explicitly");
    assert_eq!(err.code, ErrorCode(-32603), "internal_error");
    assert!(
        err.message.contains("forge.state_type_mismatch"),
        "error must carry the machine-readable error_id, got: {}",
        err.message
    );
    assert!(
        tool.seen_state
            .lock()
            .expect("mutex not poisoned")
            .is_empty(),
        "tool must not run on a downcast failure"
    );
}

/// The default `call_with_ctx` delegates to `call` — a tool that only
/// implements `call` dispatches identically through the ctx-aware chain.
#[test]
fn default_call_with_ctx_delegates_to_call() {
    let server = SdForgeMcpServer::with_tools(vec![McpToolInstance::new(
        super::create_test_tool(),
        super::create_test_metadata(),
    )]);
    let result = server
        .call_tool_internal("test", None)
        .expect("default delegation must keep plain tools dispatching");
    assert_ne!(result.is_error, Some(true));
}

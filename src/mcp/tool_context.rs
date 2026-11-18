// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Per-call context bridging MCP server wiring to tool dispatch.
//!
//! [`ToolCallContext`] carries what the plain [`SdForgeTool::call`] entry
//! point cannot:
//!
//! - **Application state**: injected once via `SdForgeMcpServer::with_state`
//!   (mirroring the gRPC `with_state` wiring) and recovered inside the tool
//!   via `crate::core::downcast_state` — the `#[state]` parameters of
//!   `#[forge(tool_name = ...)]` endpoints depend on it.
//! - **Verified identity** (feature = `security`): the `AuthContext` the
//!   verifier produced, previously consumed only by endpoint RBAC and
//!   dropped before dispatch.
//! - **Transport credentials** (module docs of
//!   `McpCredentials` for the injection contract; feature = `security`).

use crate::core::HandlerState;

/// Per-call context handed to [`SdForgeTool::call_with_ctx`].
///
/// Tools never construct this struct — the server builds it per call and
/// hands it down the dispatch chain. Hand-written tools that only implement
/// `call` are unaffected: the default `call_with_ctx` delegates to `call`.
#[derive(Clone, Default)]
pub struct ToolCallContext {
    /// Application state; `None` unless injected via
    /// `SdForgeMcpServer::with_state`.
    pub(super) state: HandlerState,
    /// Verified identity (`None` without a verifier or unauthenticated).
    #[cfg(feature = "security-core")]
    pub(super) auth: Option<crate::security::AuthContext>,
    /// Raw transport credentials as injected by the transport adapter.
    #[cfg(feature = "security-core")]
    pub(super) credentials: Option<super::server::McpCredentials>,
}

impl ToolCallContext {
    /// Clone of the application state (`None` unless the server was built
    /// via `SdForgeMcpServer::with_state`); recover the concrete type with
    /// `crate::core::downcast_state`.
    #[must_use]
    pub fn state(&self) -> HandlerState {
        self.state.clone()
    }

    /// The verified caller identity, if any (feature = `security`).
    #[cfg(feature = "security-core")]
    #[must_use]
    pub fn auth(&self) -> Option<&crate::security::AuthContext> {
        self.auth.as_ref()
    }

    /// Convenience accessor: the authenticated user id, if any
    /// (feature = `security`).
    #[cfg(feature = "security-core")]
    #[must_use]
    pub fn user_id(&self) -> Option<&str> {
        self.auth.as_ref().and_then(|a| a.user_id())
    }

    /// The raw transport credentials, if any (feature = `security`).
    #[cfg(feature = "security-core")]
    #[must_use]
    pub fn credentials(&self) -> Option<&super::server::McpCredentials> {
        self.credentials.as_ref()
    }
}

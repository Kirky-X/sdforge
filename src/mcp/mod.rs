// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! MCP server implementation built on the official rmcp SDK.
//!
//! This module provides MCP (Model Context Protocol) server functionality using
//! the official `rmcp` crate (v2.1+). It supports:
//!
//! - Compile-time tool registration via `inventory`
//! - Stateful and stateless server modes
//! - MCP 2026-07-28 protocol adaptation (headers, cache semantics, MRTR)
//! - Integration with SDForge's `#[forge]` macro
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use sdforge::mcp::{SdForgeTool, McpToolRegistration};
//! use sdforge::core::ApiMetadata;
//!
//! struct MyTool;
//! impl SdForgeTool for MyTool {
//!     fn name(&self) -> &str { "my_tool" }
//!     fn description(&self) -> &str { "Does something useful" }
//!     fn input_schema(&self) -> serde_json::Value {
//!         serde_json::json!({"type": "object"})
//!     }
//!     fn call(&self, input: Option<serde_json::Value>) -> Result<rmcp::model::CallToolResult, rmcp::model::ErrorData> {
//!         Ok(rmcp::model::CallToolResult::success(vec![
//!             rmcp::model::Content::text("Hello from my_tool".to_string()),
//!         ]))
//!     }
//! }
//!
//! inventory::submit!(McpToolRegistration::new(
//!     "my_tool",
//!     "v1",
//!     || std::sync::Arc::new(MyTool) as std::sync::Arc<dyn SdForgeTool>,
//!     || ApiMetadata::default(),
//! ));
//! ```

use crate::core::{ApiMetadata, Registration};
use rmcp::model::ErrorData;
use serde_json::Value;
use std::sync::Arc;

/// Type alias for rmcp's `ErrorData`, kept for API compatibility with the
/// `SdForgeTool` trait's `call` method return type.
///
/// In rmcp 0.16, the error type is `ErrorData` (not `McpError`). We re-export
/// it under the `McpError` name so existing code referencing `McpError` continues
/// to work without changes.
pub type McpError = ErrorData;

// ============================================================================
// Sub-modules
// ============================================================================

// Sub-modules for MCP 2026-07-28 protocol adaptation.
pub mod cache_semantics;
pub mod headers;
pub mod mrtr;
pub mod protocol;
pub mod stateless;

// Sub-modules for server and handler (extracted for maintainability).
mod handler;
mod schema_validation;
mod server;

// Test module (only compiled in test builds).
#[cfg(test)]
mod tests;

// ============================================================================
// Re-exports of public types
// ============================================================================

pub use handler::McpToolInstance;
pub(crate) use handler::value_to_json_object_arc;
pub use headers::McpHeaderInfo;
pub use mrtr::{InputRequiredResult, MrtrSession};
pub use server::SdForgeMcpServer;
#[cfg(all(feature = "mcp", feature = "security"))]
pub use server::{MCP_UNAUTHENTICATED, McpCredentials};
pub use stateless::StatelessServerHandler;

// ============================================================================
// SdForgeTool trait — bridges SDForge tools to rmcp's ServerHandler
// ============================================================================

/// Trait for MCP tools registered via SDForge's inventory system.
///
/// Each tool provides its name, description, JSON Schema for inputs, and a
/// `call` handler that returns a `CallToolResult`. This trait replaces the
/// old `mcp_sdk::tools::Tool` trait from the unmaintained 0.0.3 crate.
pub trait SdForgeTool: Send + Sync + 'static {
    /// The tool name (must be unique across all registered tools).
    fn name(&self) -> &str;

    /// Human-readable description of what the tool does.
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's input parameters.
    fn input_schema(&self) -> Value;

    /// Execute the tool with optional JSON input.
    ///
    /// Returns `CallToolResult` on success or `McpError` on failure.
    fn call(&self, input: Option<Value>) -> Result<rmcp::model::CallToolResult, McpError>;
}

// ============================================================================
// McpToolRegistration — compile-time registration via inventory
// ============================================================================

/// MCP tool registration entry (compile-time, collected via `inventory`).
///
/// Written out by hand instead of `define_registration!` because it carries
/// the per-tool RBAC `roles` declaration (`#[forge(auth(role = "..."))]` /
/// [`McpToolRegistration::with_roles`]) — mirroring
/// `GrpcHandlerRegistration.roles`. The instance type is `Arc<dyn SdForgeTool>`
/// so tools can be shared across threads without copying.
#[derive(Debug, Clone, Copy)]
pub struct McpToolRegistration {
    /// API name
    pub name: &'static str,
    /// API version
    pub version: &'static str,
    /// Function that creates the instance at runtime
    pub create_fn: fn() -> Arc<dyn SdForgeTool>,
    /// Function that creates the metadata at runtime
    pub metadata_fn: fn() -> ApiMetadata,
    /// Endpoint RBAC roles for `tools/call` (`auth(role = ...)` /
    /// `with_roles`). Empty slice = no role requirement (authentication
    /// still applies when a verifier is wired). With the `security`
    /// feature disabled, a non-empty declaration denies every call —
    /// fail-safe, mirroring `GrpcHandlerRegistration.roles`.
    pub roles: &'static [&'static str],
}

impl McpToolRegistration {
    /// Create a new registration instance without a role requirement.
    #[must_use]
    pub const fn new(
        name: &'static str,
        version: &'static str,
        create_fn: fn() -> Arc<dyn SdForgeTool>,
        metadata_fn: fn() -> ApiMetadata,
    ) -> Self {
        Self {
            name,
            version,
            create_fn,
            metadata_fn,
            roles: &[],
        }
    }

    /// Declare endpoint RBAC roles (const so `inventory::submit!` keeps
    /// accepting the builder chain).
    #[must_use]
    pub const fn with_roles(mut self, roles: &'static [&'static str]) -> Self {
        self.roles = roles;
        self
    }
}

impl crate::core::Registration for McpToolRegistration {
    type Instance = Arc<dyn SdForgeTool>;
    type Metadata = ApiMetadata;

    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> &str {
        self.version
    }
    fn create(&self) -> Self::Instance {
        (self.create_fn)()
    }
    fn metadata(&self) -> Self::Metadata {
        (self.metadata_fn)()
    }
}

inventory::collect!(McpToolRegistration);

// ============================================================================
// get_mcp_tools — collect all registered tools from inventory
// ============================================================================

mod mcp_impl;
pub use mcp_impl::{build, get_mcp_tools, serve_stdio};

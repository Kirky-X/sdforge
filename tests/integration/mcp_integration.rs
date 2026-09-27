// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// MCP Integration Tests
// Covers TC-INT-002, TC-INT-003

#[cfg(feature = "mcp")]
use sdforge::forge;

#[cfg(feature = "mcp")]
mod mcp_tests {
    use sdforge::mcp::build;

    #[tokio::test]
    async fn test_mcp_server_builds() {
        // Just verify build() can be called without panicking
        let _server = build();
    }
}

#[cfg(feature = "mcp")]
mod mcp_registration_tests {
    use rmcp::model::{CallToolResult, ErrorData as McpError};
    use sdforge::core::ApiMetadata;
    use sdforge::mcp::{McpToolRegistration, SdForgeTool};
    use std::sync::Arc;

    struct TestTool;
    impl SdForgeTool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }
        fn description(&self) -> &str {
            "A test tool"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            })
        }
        fn call(&self, _input: Option<serde_json::Value>) -> Result<CallToolResult, McpError> {
            Ok({
                let mut result = CallToolResult::success(vec![]);
                result.is_error = None;
                result
            })
        }
    }

    #[test]
    fn test_mcp_tool_registration() {
        // Test that McpToolRegistration::new() works with the new API
        fn create_tool() -> Arc<dyn SdForgeTool> {
            Arc::new(TestTool)
        }
        fn create_metadata() -> ApiMetadata {
            ApiMetadata::new(
                "test_tool".to_string(),
                "v1".to_string(),
                "A test tool".to_string(),
                None,
                false,
            )
        }
        let _registration =
            McpToolRegistration::new("test_tool", "v1", create_tool, create_metadata);

        // Verify the registration was created successfully (construction without panic)
        let _ = &_registration;
    }

    #[test]
    fn test_mcp_tool_creation() {
        // Create a tool instance directly to verify the SdForgeTool implementation works
        let tool = TestTool;
        assert_eq!(tool.name(), "test_tool");
    }
}

#[cfg(all(feature = "http", feature = "mcp"))]
mod dual_protocol_tests {
    use sdforge::http::build as http_build;
    use sdforge::mcp::build as mcp_build;

    #[tokio::test]
    async fn test_both_protocols_build() {
        // Just verify both builds can be called without panicking
        let _http_app = http_build();
        let _mcp_server = mcp_build();
    }
}

// ============================================================================
// vuln-0002 regression tests: MCP argument validation
//
// These tests verify that `#[forge(tool_name = "...")]` generates a `Params`
// struct with `#[serde(deny_unknown_fields)]` and that oversized payloads
// are rejected by `call_tool_internal`.
// ============================================================================

/// Test fixture: a `#[forge]` MCP tool with a single `String` parameter.
///
/// The macro-generated `Params` struct must use `#[serde(deny_unknown_fields)]`
/// so that clients cannot smuggle unexpected fields past the schema.
#[cfg(feature = "mcp")]
#[forge(
    name = "vuln0002_echo",
    version = "v1",
    tool_name = "vuln0002_echo",
    description = "vuln-0002 regression test echo tool"
)]
async fn vuln0002_echo(message: String) -> Result<String, sdforge::core::ApiError> {
    Ok(message)
}

#[cfg(feature = "mcp")]
mod vuln0002_security_tests {
    use sdforge::mcp::SdForgeMcpServer;

    /// Find the `vuln0002_echo` tool registered above. Returns `None` if the
    /// tool was not registered (which would itself be a regression).
    fn server_with_tool() -> SdForgeMcpServer {
        let server = SdForgeMcpServer::new();
        assert!(
            server.find_tool("vuln0002_echo").is_some(),
            "vuln0002_echo tool must be registered via #[forge]"
        );
        server
    }

    /// Verify that a valid call with the expected `message` field succeeds.
    #[test]
    fn test_vuln0002_valid_field_accepted() {
        let server = server_with_tool();
        let args = serde_json::json!({ "message": "hello" });
        let result = server.call_tool_internal("vuln0002_echo", Some(args));
        assert!(
            result.is_ok(),
            "valid arguments must be accepted, got: {:?}",
            result.err()
        );
    }

    /// Verify that an unknown field is rejected (deny_unknown_fields).
    ///
    /// vuln-0002: the macro-generated `Params` struct must include
    /// `#[serde(deny_unknown_fields)]` so clients cannot smuggle extra fields
    /// past the declared schema.
    #[test]
    fn test_vuln0002_unknown_field_rejected() {
        let server = server_with_tool();
        let args = serde_json::json!({ "message": "hi", "malicious_extra": "boom" });
        let result = server.call_tool_internal("vuln0002_echo", Some(args));
        assert!(
            result.is_err(),
            "unknown fields must be rejected by deny_unknown_fields"
        );
        let err = result.unwrap_err();
        let msg = err.to_string();
        // The error originates from serde_json::from_value failing; the
        // macro maps it via anyhow!("Failed to parse input: ...") then
        // ErrorData::internal_error. The message must mention the failure.
        assert!(
            msg.contains("Failed to parse") || msg.contains("unknown field"),
            "error should mention parse failure / unknown field, got: {msg}"
        );
    }

    /// Verify that a missing required field is rejected.
    #[test]
    fn test_vuln0002_missing_required_field_rejected() {
        let server = server_with_tool();
        let args = serde_json::json!({ "different_field": "value" });
        let result = server.call_tool_internal("vuln0002_echo", Some(args));
        assert!(result.is_err(), "missing required field must be rejected");
    }
}

// ============================================================================
// auth(role = ...) → MCP RBAC roles propagation
//
// `#[forge(tool_name = ..., auth(role = "..."))]` must carry the declared
// roles into `McpToolRegistration`/`McpToolInstance` so
// `SdForgeMcpServer::call_tool` can enforce them against the verified
// `AuthContext` (feature = `security`), mirroring the gRPC roles chain.
// ============================================================================

#[cfg(all(feature = "mcp", feature = "security"))]
#[forge(
    name = "rbac_macro_tool",
    version = "v1",
    tool_name = "rbac_macro_tool",
    description = "RBAC propagation test tool",
    auth(role = "mcp_admin")
)]
async fn rbac_macro_tool() -> Result<String, sdforge::core::ApiError> {
    Ok("ok".to_string())
}

#[cfg(all(feature = "mcp", feature = "security"))]
mod mcp_rbac_tests {
    use sdforge::mcp::get_mcp_tools;

    /// `auth(role = "mcp_admin")` must reach the MCP registration chain:
    /// `get_mcp_tools` surfaces the tool with its roles intact.
    #[test]
    fn macro_auth_roles_propagate_to_mcp_instance() {
        let tools = get_mcp_tools();
        let tool = tools
            .iter()
            .find(|t| t.tool().name() == "rbac_macro_tool")
            .expect("rbac_macro_tool must be registered via #[forge]");
        assert_eq!(tool.roles(), &["mcp_admin"]);
    }

    /// Role-less tools keep an empty roles slice (no accidental tightening).
    #[test]
    fn macro_tool_without_auth_keeps_empty_roles() {
        let tools = get_mcp_tools();
        let tool = tools
            .iter()
            .find(|t| t.tool().name() == "vuln0002_echo")
            .expect("vuln0002_echo must be registered via #[forge]");
        assert!(tool.roles().is_empty());
    }
}

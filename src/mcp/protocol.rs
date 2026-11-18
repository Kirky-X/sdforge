// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! MCP 2026-07-28 protocol adaptation layer.
//!
//! This module provides utilities for the 2026-07-28 MCP protocol version,
//! which removes the `initialize` handshake and introduces a `server/discover`
//! endpoint for capability discovery.
//!
//! # Key Changes in 2026-07-28
//!
//! - `initialize` handshake removed → use `server/discover` instead
//! - Stateless server mode (no session context)
//! - `Mcp-Method`/`Mcp-Name` HTTP headers for routing
//! - `ttlMs`/`cacheScope` cache semantics
//! - Multi Round-Trip Requests (MRTR)
//!
//! # Discovery
//!
//! The `discover` endpoint returns server capabilities, tool list, and
//! server info in a single request — replacing the multi-step
//! `initialize` → `tools/list` flow.

use crate::mcp::SdForgeMcpServer;
use rmcp::handler::server::ServerHandler;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};

/// Discovery response containing all server information.
///
/// This is returned by the `server/discover` endpoint, combining
/// server info, capabilities, and available tools in one response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResponse {
    /// Server name and version
    pub server_info: ServerInfoDto,
    /// Server capabilities
    pub capabilities: ServerCapabilitiesDto,
    /// Available tools
    pub tools: Vec<ToolDto>,
    /// Protocol version
    pub protocol_version: String,
}

/// Serializable ServerInfo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfoDto {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
}

/// Serializable ServerCapabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilitiesDto {
    /// Whether tools are supported
    pub tools: bool,
    /// Whether resources are supported
    pub resources: bool,
    /// Whether prompts are supported
    pub prompts: bool,
    /// Whether logging is supported
    pub logging: bool,
}

impl Default for ServerCapabilitiesDto {
    fn default() -> Self {
        Self {
            tools: true,
            resources: false,
            prompts: false,
            logging: false,
        }
    }
}

/// Serializable Tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDto {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: Option<String>,
    /// JSON Schema for input
    pub input_schema: serde_json::Value,
    /// Optional security metadata (required roles / auth scheme / audit flag).
    /// Absent when the tool declares no security requirements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<ToolSecurityMetadata>,
}

/// Machine-readable security metadata attached to a tool description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSecurityMetadata {
    /// Roles required to invoke the tool.
    pub required_roles: Vec<String>,
    /// Declared auth scheme (`api_key` / `jwt` / `none`), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_scheme: Option<String>,
    /// Whether the tool participates in security auditing.
    pub audit: bool,
}

/// Build a discovery response from a server.
///
/// This collects server info, capabilities, and all registered tools
/// into a single `DiscoveryResponse` that can be returned by the
/// `server/discover` endpoint.
///
/// Uses `get_all_tools` instead of `ServerHandler::list_tools` to avoid
/// needing a `RequestContext` (which requires a live transport connection).
pub async fn build_discovery_response(server: &SdForgeMcpServer) -> DiscoveryResponse {
    let info = server.get_info();
    let tools: Vec<ToolDto> = server
        .get_all_tools()
        .iter()
        .map(|tool| {
            let mut dto = tool_to_dto(tool);
            // Derive security metadata from the contract registry by tool name.
            #[cfg(feature = "security-core")]
            for reg in inventory::iter::<crate::security::ContractRegistration>() {
                let contract = reg.0;
                if contract.endpoint == tool.name.as_ref()
                    && let Some(es) = contract.mcp
                {
                    dto.security = Some(ToolSecurityMetadata {
                        required_roles: es.roles.iter().map(|r| r.to_string()).collect(),
                        auth_scheme: Some(
                            match es.auth {
                                crate::security::AuthScheme::None => "none",
                                crate::security::AuthScheme::ApiKey => "api_key", // pragma: allowlist secret
                                crate::security::AuthScheme::Jwt => "jwt",
                            }
                            .to_string(),
                        ),
                        audit: es.auth != crate::security::AuthScheme::None || !es.roles.is_empty(),
                    });
                    break;
                }
            }
            dto
        })
        .collect();

    DiscoveryResponse {
        server_info: ServerInfoDto {
            name: info.server_info.name,
            version: info.server_info.version,
        },
        capabilities: ServerCapabilitiesDto::default(),
        tools,
        protocol_version: "2026-07-28".to_string(),
    }
}

/// Convert a rmcp `Tool` to a serializable `ToolDto`.
fn tool_to_dto(tool: &Tool) -> ToolDto {
    // tool.description is Option<Cow<'static, str>>; convert to Option<String>.
    let description = tool.description.as_ref().map(|c| c.to_string());
    // tool.input_schema is Arc<JsonObject>; convert to serde_json::Value.
    let input_schema = tool.schema_as_json_value();
    ToolDto {
        security: None,
        name: tool.name.to_string(),
        description,
        input_schema,
    }
}

/// Check if a method name is a discovery method.
pub fn is_discovery_method(method: &str) -> bool {
    method == "server/discover" || method == "discover"
}

/// Check if a method name is an initialize method (deprecated in 2026-07-28).
pub fn is_initialize_method(method: &str) -> bool {
    method == "initialize"
}

/// Get the supported protocol version.
pub fn protocol_version() -> &'static str {
    "2026-07-28"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_capabilities_dto_default() {
        let caps = ServerCapabilitiesDto::default();
        assert!(caps.tools);
        assert!(!caps.resources);
        assert!(!caps.prompts);
        assert!(!caps.logging);
    }

    #[test]
    fn test_server_info_dto_serialization() {
        let info = ServerInfoDto {
            name: "test-server".to_string(),
            version: "1.0.0".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: ServerInfoDto = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test-server");
        assert_eq!(deserialized.version, "1.0.0");
    }

    #[test]
    fn test_server_capabilities_dto_serialization() {
        let caps = ServerCapabilitiesDto {
            tools: true,
            resources: true,
            prompts: false,
            logging: true,
        };
        let json = serde_json::to_string(&caps).unwrap();
        let deserialized: ServerCapabilitiesDto = serde_json::from_str(&json).unwrap();
        assert!(deserialized.tools);
        assert!(deserialized.resources);
        assert!(!deserialized.prompts);
        assert!(deserialized.logging);
    }

    #[test]
    fn test_tool_dto_serialization() {
        let tool = ToolDto {
            security: None,
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: ToolDto = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test_tool");
        assert_eq!(deserialized.description, Some("A test tool".to_string()));
    }

    #[test]
    fn test_tool_dto_serialization_no_description() {
        let tool = ToolDto {
            security: None,
            name: "simple".to_string(),
            description: None,
            input_schema: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: ToolDto = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "simple");
        assert!(deserialized.description.is_none());
    }

    #[test]
    fn test_discovery_response_serialization() {
        let response = DiscoveryResponse {
            server_info: ServerInfoDto {
                name: "test".to_string(),
                version: "1.0".to_string(),
            },
            capabilities: ServerCapabilitiesDto::default(),
            tools: vec![],
            protocol_version: "2026-07-28".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("2026-07-28"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_is_discovery_method() {
        assert!(is_discovery_method("server/discover"));
        assert!(is_discovery_method("discover"));
        assert!(!is_discovery_method("tools/list"));
        assert!(!is_discovery_method("tools/call"));
    }

    #[test]
    fn test_is_initialize_method() {
        assert!(is_initialize_method("initialize"));
        assert!(!is_initialize_method("tools/list"));
    }

    #[test]
    fn test_protocol_version() {
        assert_eq!(protocol_version(), "2026-07-28");
    }

    #[tokio::test]
    async fn test_build_discovery_response() {
        let server = SdForgeMcpServer::new();
        let response = build_discovery_response(&server).await;
        assert!(!response.server_info.name.is_empty());
        assert!(response.capabilities.tools);
        assert!(!response.tools.is_empty());
        assert_eq!(response.protocol_version, "2026-07-28");
    }

    #[tokio::test]
    async fn test_build_discovery_response_empty_server() {
        let server = SdForgeMcpServer::empty();
        let response = build_discovery_response(&server).await;
        assert_eq!(response.server_info.name, "sdforge-mcp");
        assert_eq!(response.server_info.version, "0.2.0");
        assert!(response.tools.is_empty());
    }

    #[tokio::test]
    async fn test_build_discovery_response_has_tools() {
        let server = SdForgeMcpServer::new();
        let response = build_discovery_response(&server).await;
        let tool_names: Vec<&str> = response.tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"coverage_test_tool"));
    }

    #[test]
    fn test_tool_dto_clone() {
        let tool = ToolDto {
            security: None,
            name: "test".to_string(),
            description: Some("desc".to_string()),
            input_schema: serde_json::json!({}),
        };
        let cloned = tool.clone();
        assert_eq!(tool.name, cloned.name);
        assert_eq!(tool.description, cloned.description);
    }

    #[test]
    fn test_tool_dto_debug() {
        let tool = ToolDto {
            security: None,
            name: "test".to_string(),
            description: None,
            input_schema: serde_json::json!({}),
        };
        let debug_str = format!("{:?}", tool);
        assert!(debug_str.contains("test"));
    }
}

#[cfg(all(test, feature = "security-core"))]
mod tool_security_metadata_tests {
    use super::*;

    #[test]
    fn metadata_serializes_with_roles_and_scheme() {
        let meta = ToolSecurityMetadata {
            required_roles: vec!["admin".into()],
            auth_scheme: Some("jwt".into()),
            audit: true,
        };
        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(
            json.get("auth_scheme").and_then(|v| v.as_str()),
            Some("jwt")
        );
        assert_eq!(json.get("audit").and_then(|v| v.as_bool()), Some(true));
        assert!(json.get("required_roles").is_some());
    }

    #[test]
    fn absent_metadata_is_omitted_from_tool_dto() {
        let dto = ToolDto {
            name: "t".into(),
            description: None,
            input_schema: serde_json::json!({}),
            security: None,
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert!(json.get("security").is_none());
    }
}

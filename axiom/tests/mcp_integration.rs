// Copyright (c) 2026 Kirky.X
//! MCP integration tests
//!
//! Tests for MCP tool registration and invocation.

#[cfg(all(test, feature = "mcp"))]
mod mcp_integration_tests {
    use axiom::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct User {
        id: u64,
        name: String,
    }

    #[tokio::test]
    async fn test_mcp_server_build() {
        // Build MCP server to collect tools
        let _server = axiom::mcp::build().await;

        // If we reach here without panicking, the build succeeded
    }

    #[tokio::test]
    async fn test_api_error_to_mcp_json() {
        // Test that ApiError can be converted to MCP JSON format
        let err = ApiError::InvalidInput {
            message: "Test error".to_string(),
            field: Some("test_field".to_string()),
            value: Some(serde_json::json!("test_value")),
        };

        let mcp_json = err.to_mcp_json();

        // Verify JSON format
        let mcp_value: serde_json::Value = serde_json::from_str(&mcp_json).unwrap();
        assert!(mcp_value.is_object());
        assert!(mcp_value.get("success").is_some());
        assert!(mcp_value.get("error").is_some());
        assert_eq!(mcp_value["success"], false);
    }

    #[tokio::test]
    async fn test_api_error_not_found_to_mcp_json() {
        let err = ApiError::NotFound {
            resource: "user".to_string(),
            resource_id: Some("123".to_string()),
        };

        let mcp_json = err.to_mcp_json();

        let mcp_value: serde_json::Value = serde_json::from_str(&mcp_json).unwrap();
        assert_eq!(mcp_value["success"], false);
        assert!(mcp_value["error"]["code"] == "NOT_FOUND");
    }

    #[tokio::test]
    async fn test_api_error_internal_to_mcp_json() {
        let err = ApiError::Internal {
            message: "Internal server error".to_string(),
            error_id: "ERR-001".to_string(),
        };

        let mcp_json = err.to_mcp_json();

        let mcp_value: serde_json::Value = serde_json::from_str(&mcp_json).unwrap();
        assert_eq!(mcp_value["success"], false);
        assert!(mcp_value["error"]["code"] == "INTERNAL_ERROR");
    }

    #[tokio::test]
    async fn test_mcp_json_serialization() {
        // Test that MCP JSON can be serialized and deserialized
        let err = ApiError::validation_error("INVALID_REQUEST", "Invalid request");

        let mcp_json = err.to_mcp_json();
        let mcp_value: serde_json::Value = serde_json::from_str(&mcp_json).unwrap();

        // Verify structure
        assert!(mcp_value["error"].is_object());
        assert!(mcp_value["error"]["code"].is_string());
        assert!(mcp_value["error"]["message"].is_string());
    }

    #[tokio::test]
    async fn test_mcp_error_details() {
        let err = ApiError::AuthenticationFailed {
            reason: "Unauthorized access".to_string(),
        };

        let mcp_json = err.to_mcp_json();
        let mcp_value: serde_json::Value = serde_json::from_str(&mcp_json).unwrap();

        // Verify error details
        assert_eq!(
            mcp_value["error"]["message"],
            "Authentication failed: Unauthorized access"
        );
        assert_eq!(mcp_value["error"]["code"], "AUTHENTICATION_FAILED");
    }

    #[tokio::test]
    async fn test_mcp_response_format() {
        // Test that a successful response can be formatted correctly
        let response: Result<String, ApiError> = Ok("Success".to_string());

        let mcp_json = match response {
            Ok(data) => serde_json::json!({
                "success": true,
                "data": data
            })
            .to_string(),
            Err(err) => err.to_mcp_json(),
        };

        let mcp_value: serde_json::Value = serde_json::from_str(&mcp_json).unwrap();
        assert_eq!(mcp_value["success"], true);
        assert_eq!(mcp_value["data"], "Success");
    }

    #[tokio::test]
    async fn test_mcp_complex_response_format() {
        // Test that a complex response can be formatted correctly
        let user = User {
            id: 123,
            name: "Test User".to_string(),
        };

        let response: Result<User, ApiError> = Ok(user);

        let mcp_json = match response {
            Ok(data) => serde_json::json!({
                "success": true,
                "data": data
            })
            .to_string(),
            Err(err) => err.to_mcp_json(),
        };

        let mcp_value: serde_json::Value = serde_json::from_str(&mcp_json).unwrap();
        assert_eq!(mcp_value["success"], true);
        assert_eq!(mcp_value["data"]["id"], 123);
        assert_eq!(mcp_value["data"]["name"], "Test User");
    }
}

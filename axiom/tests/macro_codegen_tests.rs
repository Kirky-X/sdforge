// Copyright (c) 2026 Kirky.X
//! Integration Tests for Macro Behavior
//!
//! Tests that verify the behavior of macros through integration testing.

use axiom::prelude::*;

#[cfg(test)]
mod macro_behavior_tests {
    use super::*;

    /// Test: Service response serialization
    #[tokio::test]
    async fn test_response_serialization() {
        let response = ServiceResponse::success("test_data");
        let json = serde_json::to_string(&response);
        assert!(json.is_ok(), "Serialization should succeed");

        let parsed: ServiceResponse<String> = serde_json::from_str(&json.unwrap()).unwrap();
        assert!(parsed.is_success(), "Parsed response should be success");
    }

    /// Test: ApiError serialization
    #[tokio::test]
    async fn test_error_serialization() {
        let error = ApiError::NotFound {
            resource: "user".to_string(),
            resource_id: Some("123".to_string()),
        };
        let json = serde_json::to_string(&error);
        assert!(json.is_ok(), "Error serialization should succeed");

        let parsed: ApiError = serde_json::from_str(&json.unwrap()).unwrap();
        assert!(
            matches!(parsed, ApiError::NotFound { .. }),
            "Parsed error should be NotFound"
        );
    }

    /// Test: ApiError with different variants
    #[tokio::test]
    async fn test_error_variants() {
        let error1 = ApiError::NotFound {
            resource: "user".to_string(),
            resource_id: None,
        };
        let error2 = ApiError::InvalidInput {
            message: "invalid data".to_string(),
            field: Some("email".to_string()),
            value: None,
        };
        let error3 = ApiError::Internal {
            message: "server error".to_string(),
            error_id: "abc123".to_string(),
        };

        assert!(matches!(error1, ApiError::NotFound { .. }));
        assert!(matches!(error2, ApiError::InvalidInput { .. }));
        assert!(matches!(error3, ApiError::Internal { .. }));
    }
}

#[cfg(test)]
mod response_tests {
    use super::*;

    /// Test: Service response success
    #[tokio::test]
    async fn test_response_success() {
        let response = ServiceResponse::success("test_data");
        assert!(response.is_success());
        assert_eq!(response.data(), Some(&"test_data"));
    }

    /// Test: Service response with different types
    #[tokio::test]
    async fn test_response_types() {
        let response_string = ServiceResponse::success("string_data");
        let response_number = ServiceResponse::success(42);
        let response_bool = ServiceResponse::success(true);

        assert!(response_string.is_success());
        assert!(response_number.is_success());
        assert!(response_bool.is_success());
    }

    /// Test: Service response with JSON data
    #[tokio::test]
    async fn test_response_json_data() {
        let json_data = serde_json::json!({
            "name": "test",
            "value": 123
        });
        let response = ServiceResponse::success(json_data.clone());
        assert!(response.is_success());
        assert_eq!(response.data(), Some(&json_data));
    }
}

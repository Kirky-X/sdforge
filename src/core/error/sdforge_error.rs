// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use thiserror::Error;

use crate::core::error::api_error::ApiError;
use crate::core::error::context::ErrorCategory;
use crate::core::response::ServiceError;

/// Unified framework error type that wraps all SDForge errors
///
/// This enum provides a single error type for the entire framework,
/// making error handling more consistent and ergonomic.
#[derive(Debug, Error)]
pub enum SdForgeError {
    /// API error - request processing failure
    #[error(transparent)]
    Api(#[from] ApiError),

    /// Authentication error
    #[cfg(feature = "security")]
    #[error(transparent)]
    Auth(#[from] crate::security::AuthError),

    /// JWT error
    #[cfg(feature = "security")]
    #[error(transparent)]
    Jwt(#[from] crate::security::JwtError),

    /// Authentication configuration error
    #[cfg(feature = "security")]
    #[error(transparent)]
    AuthConfig(#[from] crate::security::AuthConfigError),

    /// Configuration error
    #[cfg(feature = "http")]
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),

    /// Internal error with source
    #[error("Internal error: {0}")]
    Internal(String),
}

impl SdForgeError {
    /// Create a new Internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Get the error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            SdForgeError::Api(err) => err.category(),
            #[cfg(feature = "security")]
            SdForgeError::Auth(_) | SdForgeError::Jwt(_) => ErrorCategory::AuthError,
            #[cfg(feature = "security")]
            SdForgeError::AuthConfig(_) => ErrorCategory::AuthError,
            #[cfg(feature = "http")]
            SdForgeError::Config(_) => ErrorCategory::ClientError,
            SdForgeError::Internal(_) => ErrorCategory::ServerError,
        }
    }

    /// Get a sanitized error message for external display
    pub fn sanitized_message(&self) -> String {
        match self {
            SdForgeError::Api(err) => err.sanitized_message(),
            SdForgeError::Internal(msg) => msg.clone(),
            #[cfg(any(feature = "http", feature = "security"))]
            other => other.to_string(),
        }
    }

    /// Convert to ServiceError for HTTP response
    pub fn to_service_error(&self) -> ServiceError {
        match self {
            SdForgeError::Api(err) => err.to_service_error(),
            #[cfg(feature = "security")]
            SdForgeError::Auth(e) => ServiceError::with_details(
                "AUTH_ERROR",
                e.to_string(),
                serde_json::json!({ "type": "auth" }),
                401,
            ),
            #[cfg(feature = "security")]
            SdForgeError::Jwt(e) => ServiceError::with_details(
                "JWT_ERROR",
                e.to_string(),
                serde_json::json!({ "type": "jwt" }),
                401,
            ),
            #[cfg(feature = "security")]
            SdForgeError::AuthConfig(e) => ServiceError::with_details(
                "AUTH_CONFIG_ERROR",
                e.to_string(),
                serde_json::json!({ "type": "auth_config" }),
                500,
            ),
            #[cfg(feature = "http")]
            SdForgeError::Config(e) => ServiceError::with_details(
                "CONFIG_ERROR",
                e.to_string(),
                serde_json::json!({ "type": "config" }),
                400,
            ),
            SdForgeError::Internal(msg) => ServiceError::with_details(
                "INTERNAL_ERROR",
                msg.clone(),
                serde_json::json!({ "type": "internal" }),
                500,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::api_error::ApiError;
    use crate::core::error::context::ErrorCategory;

    /// Test SdForgeError::internal() constructor creates the Internal variant
    /// with the provided message.
    #[test]
    fn test_internal_constructor_with_str() {
        let err = SdForgeError::internal("something went wrong");
        match err {
            SdForgeError::Internal(msg) => {
                assert_eq!(msg, "something went wrong");
            }
            _ => panic!("Expected Internal variant"),
        }
    }

    /// Test SdForgeError::internal() accepts a String as well as &str.
    #[test]
    fn test_internal_constructor_with_string() {
        let msg = String::from("dynamic error");
        let err = SdForgeError::internal(msg);
        match err {
            SdForgeError::Internal(m) => assert_eq!(m, "dynamic error"),
            _ => panic!("Expected Internal variant"),
        }
    }

    /// Test category() for Api::NotFound returns ClientError.
    #[test]
    fn test_category_api_not_found() {
        let err = SdForgeError::from(ApiError::not_found("User", None));
        assert_eq!(err.category(), ErrorCategory::ClientError);
    }

    /// Test category() for Api::AuthenticationFailed returns AuthError.
    #[test]
    fn test_category_api_auth() {
        let err = SdForgeError::from(ApiError::authentication_failed("bad token"));
        assert_eq!(err.category(), ErrorCategory::AuthError);
    }

    /// Test category() for Api::RateLimitExceeded returns RateLimitError.
    #[test]
    fn test_category_api_rate_limit() {
        let err = SdForgeError::from(ApiError::rate_limit_exceeded(100, 60));
        assert_eq!(err.category(), ErrorCategory::RateLimitError);
    }

    /// Test category() for Api::Internal returns ServerError.
    #[test]
    fn test_category_api_internal() {
        let err = SdForgeError::from(ApiError::internal_error("boom", "err-1"));
        assert_eq!(err.category(), ErrorCategory::ServerError);
    }

    /// Test category() for Api::ValidationError returns ValidationError.
    #[test]
    fn test_category_api_validation() {
        let err = SdForgeError::from(ApiError::validation("email", "invalid"));
        assert_eq!(err.category(), ErrorCategory::ValidationError);
    }

    /// Test category() for the Internal variant returns ServerError.
    #[test]
    fn test_category_internal() {
        let err = SdForgeError::internal("internal error");
        assert_eq!(err.category(), ErrorCategory::ServerError);
    }

    /// Test category() for the Config variant (http feature) returns ClientError.
    #[cfg(feature = "http")]
    #[test]
    fn test_category_config() {
        let config_err = crate::config::ConfigError::FileNotFound {
            path: "/missing".to_string(),
        };
        let err = SdForgeError::from(config_err);
        assert_eq!(err.category(), ErrorCategory::ClientError);
    }

    /// Test sanitized_message() for Api::NotFound includes the resource name
    /// (client-facing errors are not sanitized).
    #[test]
    fn test_sanitized_message_api_not_found() {
        let err = SdForgeError::from(ApiError::not_found("User", None));
        let msg = err.sanitized_message();
        assert!(msg.contains("User"));
    }

    /// Test sanitized_message() for Api::Internal strips sensitive details.
    #[test]
    fn test_sanitized_message_api_internal_sanitized() {
        let err = SdForgeError::from(ApiError::internal_error("secret details", "err-1"));
        let msg = err.sanitized_message();
        // Internal errors are sanitized — the raw message must NOT leak.
        assert!(!msg.contains("secret details"));
        assert!(msg.contains("internal error"));
    }

    /// Test sanitized_message() for the Internal variant returns the raw message.
    #[test]
    fn test_sanitized_message_internal() {
        let err = SdForgeError::internal("my internal message");
        let msg = err.sanitized_message();
        assert_eq!(msg, "my internal message");
    }

    /// Test sanitized_message() for the Config variant (http feature).
    #[cfg(feature = "http")]
    #[test]
    fn test_sanitized_message_config() {
        let config_err = crate::config::ConfigError::FileNotFound {
            path: "/missing".to_string(),
        };
        let err = SdForgeError::from(config_err);
        let msg = err.sanitized_message();
        assert!(msg.contains("File not found"));
    }

    /// Test to_service_error() for Api::NotFound produces a 404 ServiceError.
    #[test]
    fn test_to_service_error_api_not_found() {
        let err = SdForgeError::from(ApiError::not_found("User", Some("42".to_string())));
        let service_err = err.to_service_error();
        assert_eq!(service_err.code(), "NOT_FOUND");
        assert_eq!(service_err.http_status(), 404);
    }

    /// Test to_service_error() for Api::ValidationError produces a 422 ServiceError.
    #[test]
    fn test_to_service_error_api_validation() {
        let err = SdForgeError::from(ApiError::validation("email", "invalid format"));
        let service_err = err.to_service_error();
        assert_eq!(service_err.code(), "VALIDATION_ERROR");
        assert_eq!(service_err.http_status(), 422);
    }

    /// Test to_service_error() for the Internal variant produces a 500
    /// ServiceError carrying the original message.
    #[test]
    fn test_to_service_error_internal() {
        let err = SdForgeError::internal("custom internal message");
        let service_err = err.to_service_error();
        assert_eq!(service_err.code(), "INTERNAL_ERROR");
        assert_eq!(service_err.http_status(), 500);
        assert_eq!(service_err.message(), "custom internal message");
    }

    /// Test to_service_error() for the Config variant (http feature)
    /// produces a 400 ServiceError.
    #[cfg(feature = "http")]
    #[test]
    fn test_to_service_error_config() {
        let config_err = crate::config::ConfigError::FileNotFound {
            path: "/missing".to_string(),
        };
        let err = SdForgeError::from(config_err);
        let service_err = err.to_service_error();
        assert_eq!(service_err.code(), "CONFIG_ERROR");
        assert_eq!(service_err.http_status(), 400);
    }

    /// Test From<ApiError> conversion produces the Api variant.
    #[test]
    fn test_from_api_error() {
        let api_err = ApiError::not_found("Resource", None);
        let sdforge_err: SdForgeError = api_err.into();
        match sdforge_err {
            SdForgeError::Api(ApiError::NotFound { resource, .. }) => {
                assert_eq!(resource, "Resource");
            }
            _ => panic!("Expected Api variant"),
        }
    }

    /// Test From<ConfigError> conversion (http feature) produces the Config variant.
    #[cfg(feature = "http")]
    #[test]
    fn test_from_config_error() {
        let config_err = crate::config::ConfigError::FileNotFound {
            path: "/test".to_string(),
        };
        let sdforge_err: SdForgeError = config_err.into();
        match sdforge_err {
            SdForgeError::Config(_) => {}
            _ => panic!("Expected Config variant"),
        }
    }

    /// Test Debug formatting for the Internal variant.
    #[test]
    fn test_debug_format_internal() {
        let err = SdForgeError::internal("debug message");
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Internal"));
        assert!(debug_str.contains("debug message"));
    }

    /// Test Display (Error trait) for the Internal variant.
    #[test]
    fn test_display_internal() {
        let err = SdForgeError::internal("display message");
        let s = err.to_string();
        assert_eq!(s, "Internal error: display message");
    }

    /// Test Display (Error trait) for the Api variant delegates to ApiError.
    #[test]
    fn test_display_api() {
        let err = SdForgeError::from(ApiError::not_found("User", None));
        let s = err.to_string();
        assert!(s.contains("Resource not found"));
        assert!(s.contains("User"));
    }

    /// Test Display (Error trait) for the Config variant (http feature).
    #[cfg(feature = "http")]
    #[test]
    fn test_display_config() {
        let config_err = crate::config::ConfigError::FileNotFound {
            path: "/missing".to_string(),
        };
        let err = SdForgeError::from(config_err);
        let s = err.to_string();
        assert!(s.contains("File not found"));
        assert!(s.contains("/missing"));
    }

    /// Test that SdForgeError implements Send + Sync (required for use in
    /// async contexts and Axum handlers).
    #[test]
    fn test_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SdForgeError>();
    }

    /// Test the full error chain: ApiError -> SdForgeError -> ServiceError
    /// preserves the HTTP status code and error code.
    #[test]
    fn test_error_chain_preserves_status_and_code() {
        let api_err = ApiError::access_denied("read", Some("user-1".to_string()));
        let sdforge_err: SdForgeError = api_err.into();
        let service_err = sdforge_err.to_service_error();
        assert_eq!(service_err.code(), "ACCESS_DENIED");
        assert_eq!(service_err.http_status(), 403);
    }

    // ============================================================================
    // Security error variant tests
    //
    // The to_service_error() method has dedicated branches for Auth, Jwt, and
    // AuthConfig errors (gated behind the "security" feature). These tests
    // cover those branches and verify the resulting ServiceError has the
    // correct error code, HTTP status, and JSON details.
    // ============================================================================

    /// Test to_service_error() for the Auth variant produces a 401 ServiceError
    /// with the AUTH_ERROR code and auth type marker.
    #[cfg(feature = "security")]
    #[test]
    fn test_to_service_error_auth() {
        use crate::security::AuthError;
        let auth_err = AuthError::InvalidToken;
        let sdforge_err: SdForgeError = auth_err.into();
        let service_err = sdforge_err.to_service_error();
        assert_eq!(service_err.code(), "AUTH_ERROR");
        assert_eq!(service_err.http_status(), 401);
    }

    /// Test to_service_error() for the Jwt variant produces a 401 ServiceError
    /// with the JWT_ERROR code and jwt type marker.
    #[cfg(feature = "security")]
    #[test]
    fn test_to_service_error_jwt() {
        use crate::security::JwtError;
        let jwt_err = JwtError::Expired;
        let sdforge_err: SdForgeError = jwt_err.into();
        let service_err = sdforge_err.to_service_error();
        assert_eq!(service_err.code(), "JWT_ERROR");
        assert_eq!(service_err.http_status(), 401);
    }

    /// Test to_service_error() for the AuthConfig variant produces a 500
    /// ServiceError with the AUTH_CONFIG_ERROR code and auth_config type marker.
    #[cfg(feature = "security")]
    #[test]
    fn test_to_service_error_auth_config() {
        use crate::security::AuthConfigError;
        let config_err = AuthConfigError::SecretTooShort { length: 10 };
        let sdforge_err: SdForgeError = config_err.into();
        let service_err = sdforge_err.to_service_error();
        assert_eq!(service_err.code(), "AUTH_CONFIG_ERROR");
        assert_eq!(service_err.http_status(), 500);
    }

    /// Test category() for the Auth variant returns AuthError.
    #[cfg(feature = "security")]
    #[test]
    fn test_category_auth() {
        use crate::security::AuthError;
        let err: SdForgeError = AuthError::MissingAuth.into();
        assert_eq!(err.category(), ErrorCategory::AuthError);
    }

    /// Test category() for the Jwt variant returns AuthError.
    #[cfg(feature = "security")]
    #[test]
    fn test_category_jwt() {
        use crate::security::JwtError;
        let err: SdForgeError = JwtError::InvalidSignature.into();
        assert_eq!(err.category(), ErrorCategory::AuthError);
    }

    /// Test category() for the AuthConfig variant returns AuthError.
    #[cfg(feature = "security")]
    #[test]
    fn test_category_auth_config() {
        use crate::security::AuthConfigError;
        let err: SdForgeError = AuthConfigError::InvalidSecret("weak".to_string()).into();
        assert_eq!(err.category(), ErrorCategory::AuthError);
    }

    /// Test sanitized_message() for the Auth variant (security feature).
    #[cfg(feature = "security")]
    #[test]
    fn test_sanitized_message_auth() {
        use crate::security::AuthError;
        let err: SdForgeError = AuthError::InvalidToken.into();
        let msg = err.sanitized_message();
        // Auth errors delegate to Display, which includes the error message.
        assert!(msg.contains("Invalid or expired token"));
    }

    /// Test Display (Error trait) for the Auth variant delegates to AuthError.
    #[cfg(feature = "security")]
    #[test]
    fn test_display_auth() {
        use crate::security::AuthError;
        let err: SdForgeError = AuthError::MissingAuth.into();
        let s = err.to_string();
        assert!(s.contains("Missing or invalid authorization header"));
    }

    /// Test From<AuthError> conversion produces the Auth variant.
    #[cfg(feature = "security")]
    #[test]
    fn test_from_auth_error() {
        use crate::security::AuthError;
        let auth_err = AuthError::InvalidToken;
        let sdforge_err: SdForgeError = auth_err.into();
        match sdforge_err {
            SdForgeError::Auth(_) => {}
            other => panic!("Expected Auth variant, got: {:?}", other),
        }
    }

    /// Test From<JwtError> conversion produces the Jwt variant.
    #[cfg(feature = "security")]
    #[test]
    fn test_from_jwt_error() {
        use crate::security::JwtError;
        let jwt_err = JwtError::Expired;
        let sdforge_err: SdForgeError = jwt_err.into();
        match sdforge_err {
            SdForgeError::Jwt(_) => {}
            other => panic!("Expected Jwt variant, got: {:?}", other),
        }
    }

    /// Test From<AuthConfigError> conversion produces the AuthConfig variant.
    #[cfg(feature = "security")]
    #[test]
    fn test_from_auth_config_error() {
        use crate::security::AuthConfigError;
        let config_err = AuthConfigError::SecretTooShort { length: 5 };
        let sdforge_err: SdForgeError = config_err.into();
        match sdforge_err {
            SdForgeError::AuthConfig(_) => {}
            other => panic!("Expected AuthConfig variant, got: {:?}", other),
        }
    }
}

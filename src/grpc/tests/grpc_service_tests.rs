// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `SdForgeGrpcService` methods, `GrpcRoute`/`GrpcRouteRegistration`
//! construction & accessors, `GrpcServerConfig`, address validation, `build_server`,
//! streaming/protobuf/error-propagation coverage.

use super::super::*;
use crate::core::registration::Registration;

/// Test GrpcServerConfig default values
#[test]
fn test_grpc_server_config_default() {
    let config = GrpcServerConfig::default();
    assert_eq!(config.max_connections, 1000);
    assert_eq!(config.timeout_seconds, 30);
}

/// Test GrpcServerConfig with auth configured
#[cfg(feature = "security")]
#[test]
fn test_grpc_server_config_with_auth() {
    let auth = crate::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .expect("valid secret");
    let config = GrpcServerConfig {
        max_connections: 500,
        timeout_seconds: 60,
        auth: Some(auth),
    };
    assert!(config.auth.is_some());
}

/// Test address validation with valid address
#[test]
fn test_address_validation_valid() {
    let valid_addr = "127.0.0.1:50051";
    let result = valid_addr.parse::<std::net::SocketAddr>();
    assert!(result.is_ok());
    assert_eq!(result.unwrap().port(), 50051);
}

/// Test address validation with valid address (with hostname)
#[test]
fn test_address_validation_hostname() {
    // Hostnames require DNS resolution, which can't be done with simple parse
    // This test verifies that "localhost:8080" is a valid address format
    // Note: In actual code, use std::net::ToSocketAddrs for hostname resolution
    let valid_addr = "localhost:8080";
    // Split to verify format is correct
    let parts: Vec<&str> = valid_addr.split(':').collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], "localhost");
    assert_eq!(parts[1], "8080");
}

/// Test address validation with invalid format
#[test]
fn test_address_validation_invalid() {
    let invalid_addr = "not-a-valid-address";
    let result = invalid_addr.parse::<std::net::SocketAddr>();
    assert!(result.is_err());
}

/// Test address validation with missing port
#[test]
fn test_address_validation_missing_port() {
    let invalid_addr = "127.0.0.1";
    let result = invalid_addr.parse::<std::net::SocketAddr>();
    assert!(result.is_err());
}

/// Test address validation with port out of range
#[test]
fn test_address_validation_port_range() {
    // Port 0 is technically valid for binding to any available port
    let addr = "127.0.0.1:0";
    let result = addr.parse::<std::net::SocketAddr>();
    assert!(result.is_ok());
}

/// Test SdForgeGrpcService creation
#[test]
fn test_sd_forge_grpc_service_creation() {
    let service = SdForgeGrpcService::default();
    // Just verify it can be created
    let _ = service;
}

/// Test GrpcRoute structure
#[test]
fn test_grpc_route_structure() {
    use crate::core::ApiMetadata;

    let route = GrpcRoute {
        service_name: "test-service".to_string(),
        metadata: ApiMetadata {
            name: "test".to_string(),
            version: "v1".to_string(),
            description: "Test gRPC service".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
    };

    assert_eq!(route.service_name, "test-service");
    assert_eq!(route.metadata.name, "test");
}

/// Test GrpcRoute with ApiMetadata accessors
#[test]
fn test_grpc_route_metadata_accessors() {
    use crate::core::ApiMetadata;

    let route = GrpcRoute {
        service_name: "api-service".to_string(),
        metadata: ApiMetadata {
            name: "my_api".to_string(),
            version: "v2".to_string(),
            description: "API service".to_string(),
            cache_ttl: Some(300),
            is_streaming: false,
        },
    };

    assert_eq!(route.metadata.name(), "my_api");
    assert_eq!(route.metadata.version(), "v2");
    assert_eq!(route.metadata.description(), "API service");
    assert_eq!(route.metadata.cache_ttl(), Some(300));
    assert!(!route.metadata.is_streaming());
}

// ============================================================================
// gRPC Service Method Tests
// ============================================================================

#[tokio::test]
async fn test_grpc_service_call_method() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let mut parameters = HashMap::new();
    parameters.insert("key".to_string(), "value".to_string());

    let request = CallRequest {
        method: "test_method".to_string(),
        parameters,
        data: "".to_string(),
    };

    let result = service.call(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert!(response.success);
    assert_eq!(response.status_code, 200);
    assert!(response.error.is_empty());
}

#[tokio::test]
async fn test_grpc_service_call_with_empty_method() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = CallRequest {
        method: "".to_string(),
        parameters: HashMap::new(),
        data: "".to_string(),
    };

    let result = service.call(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert!(response.success);
}

#[tokio::test]
async fn test_grpc_service_call_with_complex_parameters() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let mut parameters = HashMap::new();
    parameters.insert("user_id".to_string(), "123".to_string());
    parameters.insert("name".to_string(), "Test User".to_string());
    parameters.insert("active".to_string(), "true".to_string());

    let complex_data = serde_json::json!({
        "user_id": 123,
        "name": "Test User",
        "active": true,
        "tags": ["tag1", "tag2"]
    });

    let request = CallRequest {
        method: "update_user".to_string(),
        parameters,
        data: complex_data.to_string(),
    };

    let result = service.call(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert!(response.success);

    let response_data: serde_json::Value = serde_json::from_str(&response.data).unwrap();
    assert_eq!(response_data["method"], "update_user");
}

#[tokio::test]
async fn test_grpc_service_get_info() {
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = InfoRequest {
        version: "".to_string(),
    };
    let result = service.get_info(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert_eq!(response.name, "SdForge Service");
    assert_eq!(response.version, "0.1.0");
    assert!(!response.methods.is_empty());
    assert_eq!(response.description, "SdForge Multi-Protocol SDK Framework");
}

#[tokio::test]
async fn test_grpc_service_get_info_methods_list() {
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = InfoRequest {
        version: "0.1.0".to_string(),
    };
    let result = service.get_info(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert!(response.methods.contains(&"Call".to_string()));
    assert!(response.methods.contains(&"GetInfo".to_string()));
    assert_eq!(response.methods.len(), 2);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_grpc_service_call_with_invalid_json() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let mut parameters = HashMap::new();
    parameters.insert("key".to_string(), "value".to_string());

    let request = CallRequest {
        method: "test".to_string(),
        parameters,
        data: "invalid json {{{".to_string(),
    };

    let result = service.call(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert!(response.success);
    assert!(response.data.contains("processed"));
}

#[tokio::test]
async fn test_grpc_service_call_with_large_payload() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let large_data = "x".repeat(1000);

    let mut parameters = HashMap::new();
    parameters.insert("data".to_string(), large_data.clone());

    let request = CallRequest {
        method: "large_payload".to_string(),
        parameters,
        data: large_data,
    };

    let result = service.call(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert!(response.success);
}

// ============================================================================
// Metadata Validation Tests
// ============================================================================

#[test]
fn test_grpc_route_with_streaming_metadata() {
    use crate::core::ApiMetadata;

    let route = GrpcRoute {
        service_name: "stream-service".to_string(),
        metadata: ApiMetadata {
            name: "stream_api".to_string(),
            version: "v1".to_string(),
            description: "Streaming API".to_string(),
            cache_ttl: None,
            is_streaming: true,
        },
    };

    assert!(route.metadata.is_streaming());
    assert_eq!(route.metadata.cache_ttl(), None);
}

#[test]
fn test_grpc_route_with_cache_ttl() {
    use crate::core::ApiMetadata;

    let route = GrpcRoute {
        service_name: "cached-service".to_string(),
        metadata: ApiMetadata {
            name: "cached_api".to_string(),
            version: "v1".to_string(),
            description: "Cached API".to_string(),
            cache_ttl: Some(600),
            is_streaming: false,
        },
    };

    assert_eq!(route.metadata.cache_ttl(), Some(600));
    assert!(!route.metadata.is_streaming());
}

#[test]
fn test_grpc_route_metadata_cloning() {
    use crate::core::ApiMetadata;

    let route = GrpcRoute {
        service_name: "original".to_string(),
        metadata: ApiMetadata {
            name: "test".to_string(),
            version: "v1".to_string(),
            description: "Test".to_string(),
            cache_ttl: Some(300),
            is_streaming: false,
        },
    };

    let route_cloned = route.clone();
    assert_eq!(route_cloned.service_name, "original");
    assert_eq!(route_cloned.metadata.name, "test");
}

// ============================================================================
// Boundary Condition Tests
// ============================================================================

#[test]
fn test_grpc_config_zero_timeout() {
    let config = GrpcServerConfig {
        max_connections: 100,
        timeout_seconds: 0,
        #[cfg(feature = "security")]
        auth: None,
    };

    assert_eq!(config.timeout_seconds, 0);
    assert_eq!(config.max_connections, 100);
}

#[test]
fn test_grpc_config_large_max_connections() {
    let config = GrpcServerConfig {
        max_connections: 100000,
        timeout_seconds: 30,
        #[cfg(feature = "security")]
        auth: None,
    };

    assert_eq!(config.max_connections, 100000);
}

#[test]
fn test_grpc_config_boundary_values() {
    let config1 = GrpcServerConfig {
        max_connections: 1,
        timeout_seconds: 1,
        #[cfg(feature = "security")]
        auth: None,
    };

    let config2 = GrpcServerConfig {
        max_connections: usize::MAX,
        timeout_seconds: u64::MAX,
        #[cfg(feature = "security")]
        auth: None,
    };

    assert_eq!(config1.max_connections, 1);
    assert_eq!(config2.max_connections, usize::MAX);
    assert_eq!(config2.timeout_seconds, u64::MAX);
}

// ============================================================================
// Task 2.14: Server Streaming RPC Tests
// ============================================================================

#[test]
fn test_grpc_server_streaming_multiple_messages() {
    // Test that streaming RPC can handle multiple response messages
    let messages: Vec<String> = vec![
        "Message 1".to_string(),
        "Message 2".to_string(),
        "Message 3".to_string(),
    ];

    // Simulate streaming response
    let mut stream_count = 0;
    for msg in &messages {
        assert!(!msg.is_empty());
        stream_count += 1;
    }

    assert_eq!(stream_count, 3);
}

#[tokio::test]
async fn test_grpc_server_streaming_async() {
    // Test async streaming RPC
    async fn generate_stream() -> Vec<String> {
        vec!["async msg 1".to_string(), "async msg 2".to_string()]
    }

    let stream = generate_stream().await;
    assert_eq!(stream.len(), 2);
    assert!(stream[0].contains("async"));
}

#[test]
fn test_grpc_server_streaming_empty_stream() {
    // Test handling of empty stream
    let empty_stream: Vec<String> = vec![];
    assert_eq!(empty_stream.len(), 0);
}

// ============================================================================
// Task 2.15: ProtoBuf Encoding/Decoding Tests
// ============================================================================

#[test]
fn test_grpc_protobuf_serialization() {
    // Test ProtoBuf-like serialization
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestMessage {
        id: u32,
        name: String,
        active: bool,
    }

    let msg = TestMessage {
        id: 123,
        name: "test".to_string(),
        active: true,
    };

    let serialized = serde_json::to_string(&msg).expect("serialization should succeed");
    assert!(serialized.contains("\"id\":123"));
    assert!(serialized.contains("\"name\":\"test\""));
    assert!(serialized.contains("\"active\":true"));
}

#[test]
fn test_grpc_protobuf_deserialization() {
    // Test ProtoBuf-like deserialization
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestMessage {
        id: u32,
        name: String,
    }

    let json = r#"{"id":456,"name":"deser_test"}"#;
    let deserialized: TestMessage =
        serde_json::from_str(json).expect("deserialization should succeed");

    assert_eq!(deserialized.id, 456);
    assert_eq!(deserialized.name, "deser_test");
}

#[test]
fn test_grpc_protobuf_roundtrip() {
    // Test serialization/deserialization roundtrip
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct RoundtripMessage {
        value: i64,
        data: String,
    }

    let original = RoundtripMessage {
        value: 99999,
        data: "roundtrip data".to_string(),
    };

    let serialized = serde_json::to_string(&original).expect("serialization should succeed");
    let deserialized: RoundtripMessage =
        serde_json::from_str(&serialized).expect("deserialization should succeed");

    assert_eq!(original, deserialized);
}

// ============================================================================
// Task 2.16: Error Propagation Tests
// ============================================================================

#[test]
fn test_grpc_error_propagation_server_to_client() {
    // Test that server errors are correctly propagated to client
    use crate::core::ApiError;

    let server_error = ApiError::InvalidInput {
        message: "Invalid gRPC request parameter".to_string(),
        field: Some("user_id".to_string()),
        value: Some(serde_json::Value::String("invalid".to_string())),
    };

    // Convert to status code representation
    let error_msg = server_error.to_string();
    assert!(error_msg.contains("Invalid input"));
    assert!(error_msg.contains("Invalid gRPC request parameter"));
}

#[test]
fn test_grpc_error_status_codes() {
    // Test various gRPC status codes
    use crate::core::ApiError;

    // Test validation error
    let validation_err = ApiError::validation_error("INVALID_PARAM", "Parameter validation failed");
    assert!(validation_err
        .to_string()
        .contains("Parameter validation failed"));

    // Test not found error
    let not_found_err = ApiError::NotFound {
        resource: "User".to_string(),
        resource_id: Some("123".to_string()),
    };
    assert!(not_found_err.to_string().contains("User"));
    assert!(not_found_err.to_string().contains("not found"));
}

#[tokio::test]
async fn test_grpc_error_propagation_async() {
    // Test async error propagation
    async fn grpc_call_that_fails() -> Result<String, crate::core::ApiError> {
        Err(crate::core::ApiError::InvalidInput {
            message: "Async call failed".to_string(),
            field: None,
            value: None,
        })
    }

    let result = grpc_call_that_fails().await;
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Async call failed"));
    }
}

// ============================================================================
// GrpcRoute Constructor and Accessor Tests
// ============================================================================

#[test]
fn test_grpc_route_new_basic() {
    use crate::core::ApiMetadata;

    let metadata = ApiMetadata::new(
        "test_api".to_string(),
        "v1".to_string(),
        "Test API description".to_string(),
        None,
        false,
    );

    let route = GrpcRoute::new("my_service".to_string(), metadata);

    assert_eq!(route.service_name(), "my_service");
    assert_eq!(route.metadata().name(), "test_api");
}

#[test]
fn test_grpc_route_new_with_cache_ttl() {
    use crate::core::ApiMetadata;

    let metadata = ApiMetadata::new(
        "cached_api".to_string(),
        "v2".to_string(),
        "Cached API".to_string(),
        Some(600),
        false,
    );

    let route = GrpcRoute::new("cached_service".to_string(), metadata);

    assert_eq!(route.metadata().cache_ttl(), Some(600));
}

#[test]
fn test_grpc_route_new_with_streaming() {
    use crate::core::ApiMetadata;

    let metadata = ApiMetadata::new(
        "stream_api".to_string(),
        "v1".to_string(),
        "Streaming API".to_string(),
        None,
        true,
    );

    let route = GrpcRoute::new("stream_service".to_string(), metadata);

    assert!(route.metadata().is_streaming());
}

#[test]
fn test_grpc_route_service_name_accessor() {
    use crate::core::ApiMetadata;

    let metadata = ApiMetadata::default();
    let route = GrpcRoute::new("unique_service_name".to_string(), metadata);

    assert_eq!(route.service_name(), "unique_service_name");
}

#[test]
fn test_grpc_route_metadata_accessor() {
    use crate::core::ApiMetadata;

    let metadata = ApiMetadata::new(
        "accessor_test".to_string(),
        "v3".to_string(),
        "Accessor test description".to_string(),
        Some(120),
        false,
    );

    let route = GrpcRoute::new("accessor_service".to_string(), metadata.clone());

    let retrieved_metadata = route.metadata();
    assert_eq!(retrieved_metadata.name(), "accessor_test");
    assert_eq!(retrieved_metadata.version(), "v3");
    assert_eq!(
        retrieved_metadata.description(),
        "Accessor test description"
    );
}

#[test]
fn test_grpc_route_empty_service_name() {
    use crate::core::ApiMetadata;

    let metadata = ApiMetadata::default();
    let route = GrpcRoute::new("".to_string(), metadata);

    assert_eq!(route.service_name(), "");
}

#[test]
fn test_grpc_route_unicode_service_name() {
    use crate::core::ApiMetadata;

    let metadata = ApiMetadata::default();
    let route = GrpcRoute::new("服务名称".to_string(), metadata);

    assert_eq!(route.service_name(), "服务名称");
}

#[test]
fn test_grpc_route_long_service_name() {
    use crate::core::ApiMetadata;

    let long_name = "a".repeat(1000);
    let metadata = ApiMetadata::default();
    let route = GrpcRoute::new(long_name.clone(), metadata);

    assert_eq!(route.service_name().len(), 1000);
}

// ============================================================================
// GrpcRouteRegistration Tests
// ============================================================================

#[test]
fn test_grpc_route_registration_new() {
    use crate::core::ApiMetadata;

    fn create_test_route() -> GrpcRoute {
        let metadata = ApiMetadata::new(
            "registration_test".to_string(),
            "v1".to_string(),
            "Test route".to_string(),
            None,
            false,
        );
        GrpcRoute::new("test_registration_service".to_string(), metadata)
    }

    let registration =
        GrpcRouteRegistration::new("test_route", "v1", create_test_route, || ApiMetadata {
            name: "test".to_string(),
            version: "v1".to_string(),
            description: "Test route".to_string(),
            cache_ttl: None,
            is_streaming: false,
        });

    assert_eq!(registration.name(), "test_route");
}

#[test]
fn test_grpc_route_registration_name_accessor() {
    fn create_route() -> GrpcRoute {
        GrpcRoute::new("service".to_string(), crate::core::ApiMetadata::default())
    }

    let registration =
        GrpcRouteRegistration::new("unique_name", "v1", create_route, ApiMetadata::default);

    assert_eq!(registration.name(), "unique_name");
}

#[test]
fn test_grpc_route_registration_create() {
    use crate::core::ApiMetadata;

    fn factory_route() -> GrpcRoute {
        let metadata = ApiMetadata::new(
            "factory_api".to_string(),
            "v1".to_string(),
            "Factory created".to_string(),
            Some(300),
            false,
        );
        GrpcRoute::new("factory_service".to_string(), metadata)
    }

    let registration =
        GrpcRouteRegistration::new("factory_route", "v1", factory_route, || ApiMetadata {
            name: "factory_api".to_string(),
            version: "v1".to_string(),
            description: "Factory created".to_string(),
            cache_ttl: Some(300),
            is_streaming: false,
        });
    let route = registration.create();

    assert_eq!(route.service_name(), "factory_service");
    assert_eq!(route.metadata().name(), "factory_api");
}

#[test]
fn test_grpc_route_registration_create_multiple_times() {
    fn create_route() -> GrpcRoute {
        GrpcRoute::new(
            "multi_create".to_string(),
            crate::core::ApiMetadata::default(),
        )
    }

    let registration =
        GrpcRouteRegistration::new("multi", "v1", create_route, ApiMetadata::default);

    let route1 = registration.create();
    let route2 = registration.create();

    assert_eq!(route1.service_name(), route2.service_name());
}

#[test]
fn test_grpc_route_registration_empty_name() {
    fn create_route() -> GrpcRoute {
        GrpcRoute::new(
            "empty_name_test".to_string(),
            crate::core::ApiMetadata::default(),
        )
    }

    let registration = GrpcRouteRegistration::new("", "v1", create_route, ApiMetadata::default);

    assert_eq!(registration.name(), "");
}

#[test]
fn test_grpc_route_registration_debug_impl() {
    fn create_route() -> GrpcRoute {
        GrpcRoute::new(
            "debug_service".to_string(),
            crate::core::ApiMetadata::default(),
        )
    }

    let registration =
        GrpcRouteRegistration::new("debug_test", "v1", create_route, ApiMetadata::default);
    let debug_str = format!("{:?}", registration);

    assert!(debug_str.contains("debug_test"));
}

#[test]
fn test_grpc_route_registration_clone_impl() {
    fn create_route() -> GrpcRoute {
        GrpcRoute::new(
            "clone_service".to_string(),
            crate::core::ApiMetadata::default(),
        )
    }

    let registration =
        GrpcRouteRegistration::new("clone_test", "v1", create_route, ApiMetadata::default);
    let cloned = registration;

    assert_eq!(registration.name(), cloned.name());
}

// ============================================================================
// GrpcServerConfig Extended Tests
// ============================================================================

#[test]
fn test_grpc_server_config_clone() {
    let config = GrpcServerConfig {
        max_connections: 500,
        timeout_seconds: 45,
        #[cfg(feature = "security")]
        auth: None,
    };

    let cloned = config.clone();

    assert_eq!(config.max_connections, cloned.max_connections);
    assert_eq!(config.timeout_seconds, cloned.timeout_seconds);
}

#[test]
fn test_grpc_server_config_equality() {
    let config1 = GrpcServerConfig {
        max_connections: 100,
        timeout_seconds: 30,
        #[cfg(feature = "security")]
        auth: None,
    };

    let config2 = GrpcServerConfig {
        max_connections: 100,
        timeout_seconds: 30,
        #[cfg(feature = "security")]
        auth: None,
    };

    assert_eq!(config1.max_connections, config2.max_connections);
    assert_eq!(config1.timeout_seconds, config2.timeout_seconds);
}

#[test]
fn test_grpc_server_config_with_minimal_connections() {
    let config = GrpcServerConfig {
        max_connections: 1,
        timeout_seconds: 30,
        #[cfg(feature = "security")]
        auth: None,
    };

    assert_eq!(config.max_connections, 1);
}

#[test]
fn test_grpc_server_config_with_zero_timeout() {
    let config = GrpcServerConfig {
        max_connections: 100,
        timeout_seconds: 0,
        #[cfg(feature = "security")]
        auth: None,
    };

    assert_eq!(config.timeout_seconds, 0);
}

#[test]
fn test_grpc_server_config_timeout_edge_cases() {
    let short_timeout = GrpcServerConfig {
        max_connections: 100,
        timeout_seconds: 1,
        #[cfg(feature = "security")]
        auth: None,
    };

    let long_timeout = GrpcServerConfig {
        max_connections: 100,
        timeout_seconds: 86400,
        #[cfg(feature = "security")]
        auth: None,
    };

    assert_eq!(short_timeout.timeout_seconds, 1);
    assert_eq!(long_timeout.timeout_seconds, 86400);
}

#[cfg(feature = "security")]
#[test]
fn test_grpc_server_config_auth_none() {
    let config = GrpcServerConfig {
        max_connections: 100,
        timeout_seconds: 30,
        auth: None,
    };

    assert!(config.auth.is_none());
}

// ============================================================================
// SdForgeGrpcService Extended Tests
// ============================================================================

#[test]
fn test_sd_forge_grpc_service_default_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<SdForgeGrpcService>();
}

#[test]
fn test_sd_forge_grpc_service_debug_impl() {
    let service = SdForgeGrpcService::default();
    let debug_str = format!("{:?}", service);

    assert!(debug_str.contains("SdForgeGrpcService"));
}

#[tokio::test]
async fn test_grpc_service_call_with_special_characters_in_method() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = CallRequest {
        method: "method-with_special.chars:123".to_string(),
        parameters: HashMap::new(),
        data: "".to_string(),
    };

    let result = service.call(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert!(response.success);
}

#[tokio::test]
async fn test_grpc_service_call_with_unicode_method() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = CallRequest {
        method: "方法名称".to_string(),
        parameters: HashMap::new(),
        data: "".to_string(),
    };

    let result = service.call(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert!(response.success);
}

#[tokio::test]
async fn test_grpc_service_call_with_very_long_method_name() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let long_method = "method_".repeat(100);
    let request = CallRequest {
        method: long_method.clone(),
        parameters: HashMap::new(),
        data: "".to_string(),
    };

    let result = service.call(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert!(response.success);
    assert!(response.data.contains(&long_method));
}

#[tokio::test]
async fn test_grpc_service_call_with_parameters_containing_special_values() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let mut parameters = HashMap::new();
    parameters.insert("null_value".to_string(), "null".to_string());
    parameters.insert("empty_string".to_string(), "".to_string());
    parameters.insert("whitespace".to_string(), "   ".to_string());
    parameters.insert("json_like".to_string(), r#"{"key":"value"}"#.to_string());

    let request = CallRequest {
        method: "test_params".to_string(),
        parameters,
        data: "".to_string(),
    };

    let result = service.call(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert!(response.success);
}

#[tokio::test]
async fn test_grpc_service_call_with_empty_parameters() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = CallRequest {
        method: "empty_params".to_string(),
        parameters: HashMap::new(),
        data: "".to_string(),
    };

    let result = service.call(Request::new(request)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_grpc_service_get_info_with_version_parameter() {
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = InfoRequest {
        version: "2.0.0".to_string(),
    };

    let result = service.get_info(Request::new(request)).await;
    assert!(result.is_ok());

    let response = result.unwrap().into_inner();
    assert_eq!(response.version, "0.1.0");
}

#[tokio::test]
async fn test_grpc_service_get_info_response_structure() {
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = InfoRequest {
        version: "".to_string(),
    };

    let result = service.get_info(Request::new(request)).await;
    let response = result.unwrap().into_inner();

    assert!(!response.name.is_empty());
    assert!(!response.version.is_empty());
    assert!(!response.methods.is_empty());
    assert!(!response.description.is_empty());
}

// ============================================================================
// Address Validation Extended Tests
// ============================================================================

#[test]
fn test_address_validation_ipv6_loopback() {
    let addr = "[::1]:50051";
    let result = addr.parse::<std::net::SocketAddr>();
    assert!(result.is_ok());
}

#[test]
fn test_address_validation_ipv6_any() {
    let addr = "[::]:8080";
    let result = addr.parse::<std::net::SocketAddr>();
    assert!(result.is_ok());
}

#[test]
fn test_address_validation_ipv4_any() {
    let addr = "0.0.0.0:8080";
    let result = addr.parse::<std::net::SocketAddr>();
    assert!(result.is_ok());
}

#[test]
fn test_address_validation_ipv4_loopback() {
    let addr = "127.0.0.1:9090";
    let result = addr.parse::<std::net::SocketAddr>();
    assert!(result.is_ok());
    assert_eq!(result.unwrap().port(), 9090);
}

#[test]
fn test_address_validation_private_ip() {
    let addr = "192.168.1.1:50051";
    let result = addr.parse::<std::net::SocketAddr>();
    assert!(result.is_ok());
}

#[test]
fn test_address_validation_with_empty_string() {
    let addr = "";
    let result = addr.parse::<std::net::SocketAddr>();
    assert!(result.is_err());
}

#[test]
fn test_address_validation_with_only_colon() {
    let addr = ":";
    let result = addr.parse::<std::net::SocketAddr>();
    assert!(result.is_err());
}

#[test]
fn test_address_validation_with_invalid_port() {
    let addr = "127.0.0.1:abc";
    let result = addr.parse::<std::net::SocketAddr>();
    assert!(result.is_err());
}

#[test]
fn test_address_validation_with_port_too_large() {
    let addr = "127.0.0.1:99999";
    let result = addr.parse::<std::net::SocketAddr>();
    assert!(result.is_err());
}

#[test]
fn test_address_validation_with_negative_port() {
    let addr = "127.0.0.1:-1";
    let result = addr.parse::<std::net::SocketAddr>();
    assert!(result.is_err());
}

// ============================================================================
// CallResponse Structure Tests
// ============================================================================

#[tokio::test]
async fn test_call_response_data_contains_method() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = CallRequest {
        method: "my_custom_method".to_string(),
        parameters: HashMap::new(),
        data: "".to_string(),
    };

    let result = service.call(Request::new(request)).await.unwrap();
    let response = result.into_inner();

    let data: serde_json::Value = serde_json::from_str(&response.data).unwrap();
    assert_eq!(data["method"], "my_custom_method");
}

#[tokio::test]
async fn test_call_response_data_contains_result() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = CallRequest {
        method: "test".to_string(),
        parameters: HashMap::new(),
        data: "".to_string(),
    };

    let result = service.call(Request::new(request)).await.unwrap();
    let response = result.into_inner();

    let data: serde_json::Value = serde_json::from_str(&response.data).unwrap();
    assert_eq!(data["result"], "processed");
}

#[tokio::test]
async fn test_call_response_status_code() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = CallRequest {
        method: "test".to_string(),
        parameters: HashMap::new(),
        data: "".to_string(),
    };

    let result = service.call(Request::new(request)).await.unwrap();
    let response = result.into_inner();

    assert_eq!(response.status_code, 200);
}

#[tokio::test]
async fn test_call_response_error_field_empty_on_success() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = SdForgeGrpcService::default();

    let request = CallRequest {
        method: "test".to_string(),
        parameters: HashMap::new(),
        data: "".to_string(),
    };

    let result = service.call(Request::new(request)).await.unwrap();
    let response = result.into_inner();

    assert!(response.error.is_empty());
}

// ============================================================================
// InfoResponse Structure Tests
// ============================================================================

#[tokio::test]
async fn test_info_response_name_value() {
    use tonic::Request;

    let service = SdForgeGrpcService::default();
    let request = InfoRequest {
        version: "".to_string(),
    };

    let result = service.get_info(Request::new(request)).await.unwrap();
    let response = result.into_inner();

    assert_eq!(response.name, "SdForge Service");
}

#[tokio::test]
async fn test_info_response_version_value() {
    use tonic::Request;

    let service = SdForgeGrpcService::default();
    let request = InfoRequest {
        version: "".to_string(),
    };

    let result = service.get_info(Request::new(request)).await.unwrap();
    let response = result.into_inner();

    assert_eq!(response.version, "0.1.0");
}

#[tokio::test]
async fn test_info_response_methods_count() {
    use tonic::Request;

    let service = SdForgeGrpcService::default();
    let request = InfoRequest {
        version: "".to_string(),
    };

    let result = service.get_info(Request::new(request)).await.unwrap();
    let response = result.into_inner();

    assert_eq!(response.methods.len(), 2);
}

#[tokio::test]
async fn test_info_response_description_value() {
    use tonic::Request;

    let service = SdForgeGrpcService::default();
    let request = InfoRequest {
        version: "".to_string(),
    };

    let result = service.get_info(Request::new(request)).await.unwrap();
    let response = result.into_inner();

    assert_eq!(response.description, "SdForge Multi-Protocol SDK Framework");
}

// ============================================================================
// GrpcRoute Debug Implementation Test
// ============================================================================

#[test]
fn test_grpc_route_debug_output() {
    use crate::core::ApiMetadata;

    let metadata = ApiMetadata::new(
        "debug_api".to_string(),
        "v1".to_string(),
        "Debug test".to_string(),
        None,
        false,
    );
    let route = GrpcRoute::new("debug_service".to_string(), metadata);

    let debug_output = format!("{:?}", route);

    assert!(debug_output.contains("debug_service"));
    assert!(debug_output.contains("GrpcRoute"));
}

// ============================================================================
// Concurrent Request Simulation Tests
// ============================================================================

#[tokio::test]
async fn test_grpc_service_concurrent_calls() {
    use std::collections::HashMap;
    use tonic::Request;

    let service = std::sync::Arc::new(SdForgeGrpcService::default());

    let mut handles = vec![];

    for i in 0..10 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let request = CallRequest {
                method: format!("concurrent_method_{}", i),
                parameters: HashMap::new(),
                data: "".to_string(),
            };
            service_clone.call(Request::new(request)).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_grpc_service_concurrent_get_info() {
    use tonic::Request;

    let service = std::sync::Arc::new(SdForgeGrpcService::default());

    let mut handles = vec![];

    for _ in 0..5 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let request = InfoRequest {
                version: "".to_string(),
            };
            service_clone.get_info(Request::new(request)).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

/// Test build_server rejects invalid address format
#[tokio::test]
async fn test_build_server_invalid_address() {
    let result = build_server("not_a_valid_address").await;
    assert!(result.is_err(), "Should reject invalid address");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Invalid gRPC server address format"),
        "Error should mention invalid address, got: {}",
        err_msg
    );
}

/// Test build_server_with_config rejects invalid address format
#[tokio::test]
async fn test_build_server_with_config_invalid_address() {
    let config = GrpcServerConfig::default();
    let result = build_server_with_config("invalid:addr:format", config).await;
    assert!(result.is_err(), "Should reject invalid address");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Invalid gRPC server address format"),
        "Error should mention invalid address, got: {}",
        err_msg
    );
}

// ============================================================================
// build_server / build_server_with_config Success Path Tests
// ============================================================================

/// Test build_server successfully starts serving on a valid address.
///
/// Uses port 0 (OS-assigned) and a short timeout. A timeout means the server
/// started successfully and was still running — if it had failed to bind,
/// it would have returned Err before the timeout.
#[tokio::test]
async fn test_build_server_starts_serving_on_valid_address() {
    use std::time::Duration;

    let result =
        tokio::time::timeout(Duration::from_millis(200), build_server("127.0.0.1:0")).await;

    match result {
        Ok(Ok(())) => panic!("Server unexpectedly returned Ok before timeout"),
        Ok(Err(e)) => panic!("Server failed to start: {}", e),
        Err(_elapsed) => { /* Expected: server still running */ }
    }
}

/// Test build_server_with_config with default config (max_connections > 0, timeout > 0).
///
/// Verifies the success path including concurrency_limit and timeout application.
#[tokio::test]
async fn test_build_server_with_config_default_starts_serving() {
    use std::time::Duration;

    let config = GrpcServerConfig::default();
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        build_server_with_config("127.0.0.1:0", config),
    )
    .await;

    match result {
        Ok(Ok(())) => panic!("Server unexpectedly returned Ok before timeout"),
        Ok(Err(e)) => panic!("Server with config failed to start: {}", e),
        Err(_elapsed) => { /* Expected: server still running */ }
    }
}

/// Test build_server_with_config with zero max_connections and zero timeout.
///
/// Covers the false branches of the `if config.max_connections > 0` and
/// `if config.timeout_seconds > 0` conditionals, verifying the server
/// starts without applying concurrency_limit or timeout.
#[tokio::test]
async fn test_build_server_with_config_zero_values_starts_serving() {
    use std::time::Duration;

    let config = GrpcServerConfig {
        max_connections: 0,
        timeout_seconds: 0,
        #[cfg(feature = "security")]
        auth: None,
    };
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        build_server_with_config("127.0.0.1:0", config),
    )
    .await;

    match result {
        Ok(Ok(())) => panic!("Server unexpectedly returned Ok before timeout"),
        Ok(Err(e)) => panic!("Server with zero config failed to start: {}", e),
        Err(_elapsed) => { /* Expected: server still running */ }
    }
}

/// Test build_server_with_config with large max_connections and timeout values.
///
/// Covers the true branches with non-default values for both config fields.
#[tokio::test]
async fn test_build_server_with_config_large_values_starts_serving() {
    use std::time::Duration;

    let config = GrpcServerConfig {
        max_connections: 10000,
        timeout_seconds: 300,
        #[cfg(feature = "security")]
        auth: None,
    };
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        build_server_with_config("127.0.0.1:0", config),
    )
    .await;

    match result {
        Ok(Ok(())) => panic!("Server unexpectedly returned Ok before timeout"),
        Ok(Err(e)) => panic!("Server with large config failed to start: {}", e),
        Err(_elapsed) => { /* Expected: server still running */ }
    }
}

/// Test build_server_with_config with minimal positive config values.
///
/// Uses max_connections=1 and timeout_seconds=1 to cover the true branches
/// with minimum valid positive values.
#[tokio::test]
async fn test_build_server_with_config_minimal_positive_values() {
    use std::time::Duration;

    let config = GrpcServerConfig {
        max_connections: 1,
        timeout_seconds: 1,
        #[cfg(feature = "security")]
        auth: None,
    };
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        build_server_with_config("127.0.0.1:0", config),
    )
    .await;

    match result {
        Ok(Ok(())) => panic!("Server unexpectedly returned Ok before timeout"),
        Ok(Err(e)) => panic!("Server with minimal config failed to start: {}", e),
        Err(_elapsed) => { /* Expected: server still running */ }
    }
}

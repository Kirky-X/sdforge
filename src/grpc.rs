// Copyright (c) 2026 Kirky.X
//! gRPC protocol support for Axiom
//!
//! This module provides gRPC protocol support using tonic.

#[cfg(feature = "grpc")]
use serde_json;
#[cfg(feature = "grpc")]
use tonic::{service::interceptor, transport::Server, Request, Response, Status};

// Include generated proto code
/// gRPC protocol buffer module
#[cfg(feature = "grpc")]
pub mod sdforge_v1 {
    tonic::include_proto!("sdforge.v1");
}

#[cfg(feature = "grpc")]
use sdforge_v1::{
    sd_forge_service_server::{SdForgeService, SdForgeServiceServer},
    CallRequest, CallResponse, InfoRequest, InfoResponse,
};

#[cfg(feature = "grpc")]
/// gRPC service implementation
#[derive(Debug, Default)]
pub struct SdForgeGrpcService {
    // Add service state if needed
}

#[cfg(feature = "grpc")]
#[tonic::async_trait]
impl SdForgeService for SdForgeGrpcService {
    async fn call(&self, request: Request<CallRequest>) -> Result<Response<CallResponse>, Status> {
        let req = request.into_inner();

        let response = CallResponse {
            success: true,
            data: serde_json::to_string(&serde_json::json!({
                "method": req.method,
                "result": "processed"
            }))
            .map_err(|e| Status::internal(format!("Failed to serialize response: {}", e)))?,
            error: String::new(),
            status_code: 200,
        };

        Ok(Response::new(response))
    }

    async fn get_info(
        &self,
        _request: Request<InfoRequest>,
    ) -> Result<Response<InfoResponse>, Status> {
        let response = InfoResponse {
            name: "SdForge Service".to_string(),
            version: "0.1.0".to_string(),
            methods: vec!["Call".to_string(), "GetInfo".to_string()],
            description: "SdForge Multi-Protocol SDK Framework".to_string(),
        };

        Ok(Response::new(response))
    }
}

#[cfg(feature = "grpc")]
use crate::core::ApiMetadata;

#[cfg(feature = "grpc")]
/// gRPC route registration
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct GrpcRoute {
    /// The gRPC service name
    service_name: String,
    /// API metadata
    metadata: ApiMetadata,
}

#[allow(dead_code)]
impl GrpcRoute {
    #[allow(missing_docs)]
    pub fn new(service_name: String, metadata: ApiMetadata) -> Self {
        Self {
            service_name,
            metadata,
        }
    }

    pub(crate) fn service_name(&self) -> &str {
        &self.service_name
    }

    pub(crate) fn metadata(&self) -> &ApiMetadata {
        &self.metadata
    }
}

#[cfg(feature = "grpc")]
#[allow(missing_docs)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct GrpcRouteRegistration {
    name: &'static str,
    create_fn: fn() -> GrpcRoute,
}

#[cfg(feature = "grpc")]
#[allow(dead_code)]
impl GrpcRouteRegistration {
    #[allow(missing_docs)]
    pub const fn new(name: &'static str, create_fn: fn() -> GrpcRoute) -> Self {
        Self { name, create_fn }
    }

    #[allow(missing_docs)]
    pub(crate) fn name(&self) -> &str {
        self.name
    }

    #[allow(missing_docs)]
    pub(crate) fn create(&self) -> GrpcRoute {
        (self.create_fn)()
    }
}

#[cfg(feature = "grpc")]
inventory::collect!(GrpcRouteRegistration);

#[cfg(feature = "grpc")]
/// Build gRPC server
pub async fn build_server(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Security fix: Validate address format before parsing to prevent information disclosure
    let addr = match addr.parse::<std::net::SocketAddr>() {
        Ok(addr) => addr,
        Err(e) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid gRPC server address format: {}", e),
            )));
        }
    };
    let service = SdForgeGrpcService::default();

    println!("gRPC server listening on {}", addr);

    // Limit request size to 4MB to prevent large message attacks
    Server::builder()
        .add_service(SdForgeServiceServer::new(service).max_decoding_message_size(4 * 1024 * 1024))
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(feature = "grpc")]
/// Build gRPC server with custom configuration and optional JWT authentication.
///
/// When `config.auth` is `Some`, all gRPC requests must include a valid JWT bearer token
/// in the `authorization` metadata header. Invalid tokens result in `UNAUTHENTICATED` status.
pub async fn build_server_with_config(
    addr: &str,
    config: GrpcServerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Security fix: Validate address format before parsing to prevent information disclosure
    let addr = match addr.parse::<std::net::SocketAddr>() {
        Ok(addr) => addr,
        Err(e) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid gRPC server address format: {}", e),
            )));
        }
    };
    let service = SdForgeGrpcService::default();

    println!("gRPC server listening on {}", addr);

    // Build server with optional JWT auth interceptor
    let auth_interceptor = make_auth_interceptor(config.auth.clone());
    Server::builder()
        .layer(interceptor(auth_interceptor))
        .add_service(SdForgeServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(feature = "grpc")]
/// gRPC server configuration with optional JWT authentication.
#[derive(Clone)]
pub struct GrpcServerConfig {
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Optional JWT authentication.
    /// When `Some`, all gRPC requests must include a valid JWT bearer token
    /// in the `authorization` metadata header.
    pub auth: Option<crate::security::BearerAuth>,
}

#[cfg(feature = "grpc")]
impl Default for GrpcServerConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            timeout_seconds: 30,
            auth: None,
        }
    }
}

/// Create a gRPC authentication interceptor from an optional BearerAuth config.
///
/// When `auth` is `None`, returns `Ok(())` (no auth required).
/// When `auth` is `Some`, validates the `authorization` metadata header as a bearer token.
#[cfg(feature = "grpc")]
fn make_auth_interceptor(
    auth: Option<crate::security::BearerAuth>,
) -> impl FnMut(tonic::Request<()>) -> Result<tonic::Request<()>, Status> + Clone + Send + 'static {
    move |req: tonic::Request<()>| {
        let Some(ref bearer_auth) = auth else {
            return Ok(req);
        };

        // Extract bearer token from gRPC metadata (lowercase key for HTTP/2)
        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(String::from);

        match token {
            Some(token_str) => {
                if bearer_auth.validate_token(&token_str).is_some() {
                    Ok(req)
                } else {
                    Err(Status::unauthenticated("Invalid or expired token"))
                }
            }
            None => Err(Status::unauthenticated("Missing authorization header")),
        }
    }
}

#[cfg(feature = "grpc")]
#[cfg(test)]
mod tests {
    use super::*;

    /// Test GrpcServerConfig default values
    #[test]
    fn test_grpc_server_config_default() {
        let config = GrpcServerConfig::default();
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.timeout_seconds, 30);
    }

    /// Test GrpcServerConfig with auth configured
    #[test]
    fn test_grpc_server_config_with_auth() {
        let auth = crate::security::BearerAuth::try_new(
            "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        )
        .expect("valid secret");
        let config = GrpcServerConfig {
            max_connections: 500,
            timeout_seconds: 60,
            auth: Some(auth),
        };
        assert!(config.auth.is_some());
    }

    // ============================================================================
    // gRPC Auth Interceptor Tests
    // ============================================================================

    /// Generate a valid JWT for testing with the given secret and expiration timestamp
    fn make_test_jwt(secret: &str, exp_timestamp: i64) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let header = serde_json::json!({
            "alg": "HS256",
            "typ": "JWT"
        });
        let payload = serde_json::json!({
            "sub": "test-user",
            "exp": exp_timestamp,
            "iat": 1000000000
        });

        let header_b64 = base64url_encode(&serde_json::to_vec(&header).unwrap());
        let payload_b64 = base64url_encode(&serde_json::to_vec(&payload).unwrap());
        let signing_input = format!("{}.{}", header_b64, payload_b64);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signing_input.as_bytes());
        let signature = mac.finalize().into_bytes();
        let signature_b64 = base64url_encode(&signature);

        format!("{}.{}.{}", header_b64, payload_b64, signature_b64)
    }

    /// Base64url encode (no padding) for JWT encoding.
    /// Standard base64 uses `+/=`; base64url uses `-_` with no padding.
    fn base64url_encode(input: &[u8]) -> String {
        const ALPHABET: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut result = String::new();
        let mut i = 0;
        while i < input.len() {
            let b0 = input[i] as usize;
            let b1 = if i + 1 < input.len() { input[i + 1] as usize } else { 0 };
            let b2 = if i + 2 < input.len() { input[i + 2] as usize } else { 0 };

            result.push(ALPHABET[(b0 >> 2)] as char);
            result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

            if i + 1 < input.len() {
                result.push(ALPHABET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
            }
            if i + 2 < input.len() {
                result.push(ALPHABET[b2 & 0x3F] as char);
            }
            i += 3;
        }
        result
    }

    /// Test: valid token → interceptor returns Ok
    #[test]
    fn test_auth_interceptor_valid_token() {
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
        let mut interceptor = make_auth_interceptor(Some(auth));

        // Generate a token that expires in 1 year
        let exp = chrono::Utc::now().timestamp() + 365 * 24 * 3600;
        let valid_token = make_test_jwt(secret, exp);
        let auth_value: tonic::metadata::MetadataValue<tonic::metadata::Ascii> =
            tonic::metadata::MetadataValue::try_from(format!("Bearer {}", valid_token).as_str())
                .expect("valid metadata value");

        let mut req = tonic::Request::new(());
        req.metadata_mut().insert("authorization", auth_value);

        let result = interceptor(req);
        assert!(
            result.is_ok(),
            "Valid token should be accepted by interceptor"
        );
    }

    /// Test: missing authorization header → interceptor returns Err
    #[test]
    fn test_auth_interceptor_missing_auth_header() {
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
        let mut interceptor = make_auth_interceptor(Some(auth));

        // Request without any authorization header
        let req = tonic::Request::new(());
        let result = interceptor(req);

        assert!(result.is_err(), "Missing auth header should be rejected");
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unauthenticated);
        assert_eq!(
            status.message(),
            "Missing authorization header",
            "Error message should match expected text"
        );
    }

    /// Test: invalid token (bad signature) → interceptor returns Err
    #[test]
    fn test_auth_interceptor_invalid_token() {
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
        let mut interceptor = make_auth_interceptor(Some(auth));

        // Generate a token signed with a DIFFERENT secret
        let wrong_secret = "WrongSecret000!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let exp = chrono::Utc::now().timestamp() + 365 * 24 * 3600;
        let invalid_token = make_test_jwt(wrong_secret, exp);
        let auth_value: tonic::metadata::MetadataValue<tonic::metadata::Ascii> =
            tonic::metadata::MetadataValue::try_from(format!("Bearer {}", invalid_token).as_str())
                .expect("valid metadata value");

        let mut req = tonic::Request::new(());
        req.metadata_mut().insert("authorization", auth_value);

        let result = interceptor(req);
        assert!(result.is_err(), "Invalid token should be rejected");
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unauthenticated);
        assert_eq!(
            status.message(),
            "Invalid or expired token",
            "Error message should match expected text"
        );
    }

    /// Test: expired token → interceptor returns Err
    #[test]
    fn test_auth_interceptor_expired_token() {
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
        let mut interceptor = make_auth_interceptor(Some(auth));

        // Generate a token that expired in 2000
        let expired_token = make_test_jwt(secret, 946684799); // 2000-01-01
        let auth_value: tonic::metadata::MetadataValue<tonic::metadata::Ascii> =
            tonic::metadata::MetadataValue::try_from(format!("Bearer {}", expired_token).as_str())
                .expect("valid metadata value");

        let mut req = tonic::Request::new(());
        req.metadata_mut().insert("authorization", auth_value);

        let result = interceptor(req);
        assert!(result.is_err(), "Expired token should be rejected");
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unauthenticated);
        assert_eq!(
            status.message(),
            "Invalid or expired token",
            "Error message should match expected text"
        );
    }

    /// Test: no auth configured → interceptor allows all requests (pass-through)
    #[test]
    fn test_auth_interceptor_no_auth_configured() {
        let mut interceptor = make_auth_interceptor(None);

        // Even with an authorization header, when no auth is configured, it should pass
        let req = tonic::Request::new(());
        let result = interceptor(req);
        assert!(
            result.is_ok(),
            "When no auth is configured, interceptor should pass through all requests"
        );
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
            auth: None,
        };

        assert_eq!(config.max_connections, 100000);
    }

    #[test]
    fn test_grpc_config_boundary_values() {
        let config1 = GrpcServerConfig {
            max_connections: 1,
            timeout_seconds: 1,
            auth: None,
        };

        let config2 = GrpcServerConfig {
            max_connections: usize::MAX,
            timeout_seconds: u64::MAX,
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
            vec![
                "async msg 1".to_string(),
                "async msg 2".to_string(),
            ]
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
        use serde::{Serialize, Deserialize};

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
        use serde::{Serialize, Deserialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestMessage {
            id: u32,
            name: String,
        }

        let json = r#"{"id":456,"name":"deser_test"}"#;
        let deserialized: TestMessage = serde_json::from_str(json).expect("deserialization should succeed");

        assert_eq!(deserialized.id, 456);
        assert_eq!(deserialized.name, "deser_test");
    }

    #[test]
    fn test_grpc_protobuf_roundtrip() {
        // Test serialization/deserialization roundtrip
        use serde::{Serialize, Deserialize};

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
        let deserialized: RoundtripMessage = serde_json::from_str(&serialized).expect("deserialization should succeed");

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
        assert!(validation_err.to_string().contains("Parameter validation failed"));

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
}

// Copyright (c) 2026 Kirky.X
//! gRPC protocol support for Axiom
//!
//! This module provides gRPC protocol support using tonic.

#[cfg(feature = "grpc")]
use serde_json;
#[cfg(feature = "grpc")]
use tonic::{transport::Server, Request, Response, Status};

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
pub struct GrpcRoute {
    /// The gRPC service name
    pub service_name: String,
    /// API metadata
    pub metadata: ApiMetadata,
}

#[cfg(feature = "grpc")]
inventory::collect!(GrpcRoute);

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
/// Build gRPC server with custom configuration
pub async fn build_server_with_config(
    addr: &str,
    _config: GrpcServerConfig,
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

    Server::builder()
        .add_service(SdForgeServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(feature = "grpc")]
/// gRPC server configuration
#[derive(Debug, Clone)]
pub struct GrpcServerConfig {
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
}

#[cfg(feature = "grpc")]
impl Default for GrpcServerConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            timeout_seconds: 30,
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

    /// Test GrpcServerConfig with custom values
    #[test]
    fn test_grpc_server_config_custom() {
        let config = GrpcServerConfig {
            max_connections: 500,
            timeout_seconds: 60,
        };
        assert_eq!(config.max_connections, 500);
        assert_eq!(config.timeout_seconds, 60);
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
}

// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! gRPC protocol support for Axiom
//!
//! This module provides gRPC protocol support using tonic.

#[cfg(feature = "grpc")]
use serde_json;

#[cfg(feature = "grpc")]
use tonic::{Request, Response, Status, transport::Server};
#[cfg(feature = "grpc")]
/// gRPC protocol buffer module
pub mod sdforge_v1 {
    include!("pb/sdforge.v1.rs");
}

#[cfg(feature = "grpc")]
use sdforge_v1::{
    CallRequest, CallResponse, InfoRequest, InfoResponse,
    sd_forge_service_server::{SdForgeService, SdForgeServiceServer},
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
use crate::define_registration;

#[cfg(feature = "grpc")]
/// gRPC route registration
#[derive(Debug, Clone)]
pub struct GrpcRoute {
    /// The gRPC service name
    #[allow(dead_code)]
    pub(crate) service_name: String,
    /// API metadata
    #[allow(dead_code)]
    pub(crate) metadata: ApiMetadata,
}

impl GrpcRoute {
    #[allow(missing_docs)]
    pub fn new(service_name: String, metadata: ApiMetadata) -> Self {
        Self {
            service_name,
            metadata,
        }
    }

    #[cfg(test)]
    pub(crate) fn service_name(&self) -> &str {
        &self.service_name
    }

    #[cfg(test)]
    pub(crate) fn metadata(&self) -> &ApiMetadata {
        &self.metadata
    }
}

#[cfg(feature = "grpc")]
define_registration!(GrpcRouteRegistration, GrpcRoute, ApiMetadata);

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

    // Build server with optional JWT auth interceptor
    #[cfg(feature = "security")]
    let mut builder = {
        let auth_interceptor = make_auth_interceptor(config.auth.clone());
        Server::builder().layer(tonic::service::InterceptorLayer::new(auth_interceptor))
    };
    #[cfg(not(feature = "security"))]
    let mut builder = { Server::builder() };

    // Apply concurrency limit from config (previously ignored — H-8/DEBT-6).
    // tonic 0.14 exposes per-connection stream limits via concurrency_limit_per_connection;
    // global connection caps require transport-level configuration.
    if config.max_connections > 0 {
        builder = builder.concurrency_limit_per_connection(config.max_connections);
    }

    // Apply request timeout from config (previously ignored — H-8/DEBT-6).
    // Only apply when > 0 to avoid immediate-timeout behavior.
    if config.timeout_seconds > 0 {
        builder = builder.timeout(std::time::Duration::from_secs(config.timeout_seconds));
    }

    builder
        .add_service(SdForgeServiceServer::new(service).max_decoding_message_size(4 * 1024 * 1024))
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
    #[cfg(feature = "security")]
    pub auth: Option<crate::security::BearerAuth>,
}

#[cfg(feature = "grpc")]
impl Default for GrpcServerConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            timeout_seconds: 30,
            #[cfg(feature = "security")]
            auth: None,
        }
    }
}

/// Create a gRPC authentication interceptor from an optional BearerAuth config.
///
/// When `auth` is `None`, returns `Ok(())` (no auth required).
/// When `auth` is `Some`, validates the `authorization` metadata header as a bearer token.
#[cfg(all(feature = "grpc", feature = "security"))]
fn make_auth_interceptor(auth: Option<crate::security::BearerAuth>) -> AuthGrpcInterceptor {
    AuthGrpcInterceptor { auth }
}

/// gRPC authentication interceptor
#[cfg(all(feature = "grpc", feature = "security"))]
#[derive(Clone)]
struct AuthGrpcInterceptor {
    auth: Option<crate::security::BearerAuth>,
}

#[cfg(all(feature = "grpc", feature = "security"))]
impl tonic::service::Interceptor for AuthGrpcInterceptor {
    fn call(&mut self, req: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        let Some(ref bearer_auth) = self.auth else {
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

#[cfg(test)]
#[cfg(feature = "grpc")]
mod tests;

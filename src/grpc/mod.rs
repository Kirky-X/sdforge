// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! gRPC protocol support for Axiom
//!
//! This module provides gRPC protocol support using tonic.

#[cfg(feature = "grpc")]
/// gRPC protocol buffer module
pub mod sdforge_v1 {
    include!("pb/sdforge.v1.rs");
}

#[cfg(feature = "grpc")]
use crate::core::ApiMetadata;
#[cfg(feature = "grpc")]
use crate::define_registration;

mod grpc_impl;
#[cfg(feature = "grpc")]
pub use grpc_impl::{build_server, build_server_with_config};
#[cfg(all(feature = "grpc", feature = "security"))]
pub(crate) use grpc_impl::make_auth_interceptor;

#[cfg(feature = "grpc")]
/// gRPC service implementation
#[derive(Debug, Default)]
pub struct SdForgeGrpcService {
    // Add service state if needed
}

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

#[cfg(feature = "grpc")]
define_registration!(GrpcRouteRegistration, GrpcRoute, ApiMetadata);

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

/// gRPC authentication interceptor
#[cfg(all(feature = "grpc", feature = "security"))]
#[derive(Clone)]
struct AuthGrpcInterceptor {
    auth: Option<crate::security::BearerAuth>,
}

#[cfg(test)]
#[cfg(feature = "grpc")]
mod tests;

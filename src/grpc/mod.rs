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
#[cfg(all(feature = "grpc", feature = "security"))]
#[cfg(test)]
pub(crate) use grpc_impl::make_auth_interceptor;
#[cfg(feature = "grpc")]
#[allow(deprecated)]
pub use grpc_impl::{build_server, build_server_with_config, SdForgeGrpcService};

#[cfg(feature = "grpc")]
/// gRPC handler registration (links `CallRequest.method` → forge handler).
pub mod handler;
#[cfg(feature = "grpc")]
pub use handler::GrpcHandlerRegistration;

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
    /// Whether authentication is required to start the server (vuln-0006).
    ///
    /// Defaults to `true` (secure default). When `true`, `build_server_with_config`
    /// refuses to start if `auth` is `None`, preventing accidental deployment of
    /// an unauthenticated gRPC server. Set to `false` for development/test only.
    pub require_auth: bool,
    /// Optional JWT authentication.
    /// When `Some`, all gRPC requests must include a valid JWT bearer token
    /// in the `authorization` metadata header.
    #[cfg(feature = "security")]
    pub auth: Option<crate::security::BearerAuth>,
    /// Optional application state injected into `SdForgeGrpcService`.
    ///
    /// Mirrors `CliBuilder::with_dependencies`. Handlers with a `State`
    /// parameter downcast this `Arc<dyn Any>` to their concrete type at
    /// call time. Available without the `security` feature (design D5).
    pub state: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
}

/// gRPC authentication interceptor
#[cfg(all(feature = "grpc", feature = "security"))]
#[derive(Clone)]
pub(crate) struct AuthGrpcInterceptor {
    auth: Option<crate::security::BearerAuth>,
}

#[cfg(test)]
#[cfg(feature = "grpc")]
mod tests;

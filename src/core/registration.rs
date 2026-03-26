// Copyright (c) 2026 Kirky.X
//! Unified registration trait for protocol implementations
//!
//! This module provides a common interface for registering routes, services,
//! and handlers across different protocols (HTTP, MCP, WebSocket, gRPC).

use crate::core::ApiMetadata;

/// Registration result type
pub type RegistrationResult<T> = Result<T, crate::core::error::ApiError>;

/// Unified trait for protocol registrations
///
/// Implement this trait to provide a common interface for registering
/// services across different protocols.
#[allow(clippy::result_large_err)]
pub trait Registration: Send + Sync {
    /// Get the protocol name (e.g., "http", "mcp", "websocket", "grpc")
    fn protocol(&self) -> &str;

    /// Get the API metadata
    fn metadata(&self) -> &ApiMetadata;

    /// Register the service
    fn register(&self) -> RegistrationResult<()>;

    /// Get the number of registered routes/endpoints
    fn route_count(&self) -> usize;
}

/// HTTP route registration
pub mod http {
    use super::*;
    use crate::http::RouteRegistration;

    /// HTTP-specific registration
    #[derive(Debug, Clone)]
    pub struct HttpRegistration {
        routes: Vec<RouteRegistration>,
        metadata: ApiMetadata,
    }

    impl HttpRegistration {
        /// Create new HTTP registration
        pub fn new(metadata: ApiMetadata) -> Self {
            Self {
                routes: Vec::new(),
                metadata,
            }
        }

        /// Add a route
        pub fn add_route(&mut self, route: RouteRegistration) {
            self.routes.push(route);
        }
    }

    impl Registration for HttpRegistration {
        fn protocol(&self) -> &str {
            "http"
        }

        fn metadata(&self) -> &ApiMetadata {
            &self.metadata
        }

        fn register(&self) -> RegistrationResult<()> {
            // HTTP registration logic would go here
            Ok(())
        }

        fn route_count(&self) -> usize {
            self.routes.len()
        }
    }
}

/// MCP protocol registration
pub mod mcp {
    use super::*;

    /// MCP-specific registration
    #[derive(Debug, Clone)]
    pub struct McpRegistration {
        tools_count: usize,
        resources_count: usize,
        metadata: ApiMetadata,
    }

    impl McpRegistration {
        /// Create new MCP registration
        pub fn new(metadata: ApiMetadata) -> Self {
            Self {
                tools_count: 0,
                resources_count: 0,
                metadata,
            }
        }

        /// Set tools count
        pub fn with_tools(mut self, count: usize) -> Self {
            self.tools_count = count;
            self
        }

        /// Set resources count
        pub fn with_resources(mut self, count: usize) -> Self {
            self.resources_count = count;
            self
        }
    }

    impl Registration for McpRegistration {
        fn protocol(&self) -> &str {
            "mcp"
        }

        fn metadata(&self) -> &ApiMetadata {
            &self.metadata
        }

        fn register(&self) -> RegistrationResult<()> {
            Ok(())
        }

        fn route_count(&self) -> usize {
            self.tools_count + self.resources_count
        }
    }
}

/// WebSocket protocol registration
pub mod websocket {
    use super::*;

    /// WebSocket-specific registration
    #[derive(Debug, Clone)]
    pub struct WsRegistration {
        handlers_count: usize,
        metadata: ApiMetadata,
    }

    impl WsRegistration {
        /// Create new WebSocket registration
        pub fn new(metadata: ApiMetadata) -> Self {
            Self {
                handlers_count: 0,
                metadata,
            }
        }

        /// Set handlers count
        pub fn with_handlers(mut self, count: usize) -> Self {
            self.handlers_count = count;
            self
        }
    }

    impl Registration for WsRegistration {
        fn protocol(&self) -> &str {
            "websocket"
        }

        fn metadata(&self) -> &ApiMetadata {
            &self.metadata
        }

        fn register(&self) -> RegistrationResult<()> {
            Ok(())
        }

        fn route_count(&self) -> usize {
            self.handlers_count
        }
    }
}

/// gRPC protocol registration
pub mod grpc {
    use super::*;

    /// gRPC-specific registration
    #[derive(Debug, Clone)]
    pub struct GrpcRegistration {
        services_count: usize,
        metadata: ApiMetadata,
    }

    impl GrpcRegistration {
        /// Create new gRPC registration
        pub fn new(metadata: ApiMetadata) -> Self {
            Self {
                services_count: 0,
                metadata,
            }
        }

        /// Set services count
        pub fn with_services(mut self, count: usize) -> Self {
            self.services_count = count;
            self
        }
    }

    impl Registration for GrpcRegistration {
        fn protocol(&self) -> &str {
            "grpc"
        }

        fn metadata(&self) -> &ApiMetadata {
            &self.metadata
        }

        fn register(&self) -> RegistrationResult<()> {
            Ok(())
        }

        fn route_count(&self) -> usize {
            self.services_count
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ApiMetadata;

    fn test_metadata() -> ApiMetadata {
        ApiMetadata::new(
            "test".to_string(),
            "v1".to_string(),
            "Test API".to_string(),
            None,
            false,
        )
    }

    #[test]
    fn test_http_registration() {
        let reg = http::HttpRegistration::new(test_metadata());
        assert_eq!(reg.protocol(), "http");
        assert_eq!(reg.route_count(), 0);
    }

    #[test]
    fn test_mcp_registration() {
        let reg = mcp::McpRegistration::new(test_metadata())
            .with_tools(3)
            .with_resources(2);
        assert_eq!(reg.protocol(), "mcp");
        assert_eq!(reg.route_count(), 5);
    }

    #[test]
    fn test_websocket_registration() {
        let reg = websocket::WsRegistration::new(test_metadata()).with_handlers(5);
        assert_eq!(reg.protocol(), "websocket");
        assert_eq!(reg.route_count(), 5);
    }

    #[test]
    fn test_grpc_registration() {
        let reg = grpc::GrpcRegistration::new(test_metadata()).with_services(4);
        assert_eq!(reg.protocol(), "grpc");
        assert_eq!(reg.route_count(), 4);
    }
}

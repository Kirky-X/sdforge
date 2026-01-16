// Copyright (c) 2026 Kirky.X
//! Integration tests for gRPC protocol feature

#[cfg(feature = "grpc")]
mod grpc_tests {
    use axiom::grpc::{AxiomGrpcService, GrpcServerConfig};

    /// Test basic gRPC service creation
    #[tokio::test]
    async fn test_grpc_service_creation() {
        let _service = AxiomGrpcService::default();
    }

    /// Test gRPC server configuration
    #[tokio::test]
    async fn test_grpc_server_config() {
        let config = GrpcServerConfig::default();
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.timeout_seconds, 30);

        let custom_config = GrpcServerConfig {
            max_connections: 500,
            timeout_seconds: 60,
        };
        assert_eq!(custom_config.max_connections, 500);
        assert_eq!(custom_config.timeout_seconds, 60);
    }
}

#[cfg(not(feature = "grpc"))]
mod grpc_tests {
    /// Test that gRPC tests are skipped when feature is not enabled
    #[tokio::test]
    async fn test_grpc_feature_disabled() {
        // This test should pass when grpc feature is disabled
        // The actual grpc tests are conditionally compiled
    }
}

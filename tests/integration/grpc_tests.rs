#[cfg(feature = "grpc")]
mod grpc_tests {
    use sdforge::grpc::{
        SdForgeGrpcService, GrpcRoute, GrpcRouteRegistration, GrpcServerConfig,
    };
    use sdforge::core::ApiMetadata;

    #[test]
    fn test_sdforge_grpc_service_default() {
        let service = SdForgeGrpcService::default();
        // Verify it can be created
        let _ = format!("{:?}", service);
    }

    #[test]
    fn test_grpc_route_new() {
        let metadata = ApiMetadata::new(
            "test_service".to_string(),
            "v1".to_string(),
            "Test gRPC service".to_string(),
            None,
            false,
        );
        let route = GrpcRoute::new("test_service".to_string(), metadata);
        // Verify it can be created
        let _ = route;
    }

    #[test]
    fn test_grpc_route_registration_new() {
        fn create_route() -> GrpcRoute {
            let metadata = ApiMetadata::new(
                "reg_test".to_string(),
                "v1".to_string(),
                "Registration test".to_string(),
                None,
                false,
            );
            GrpcRoute::new("reg_test".to_string(), metadata)
        }
        
        let registration = GrpcRouteRegistration::new("reg_test", create_route);
        // Verify it can be created
        let _ = registration;
    }

    #[test]
    fn test_grpc_server_config_default() {
        let config = GrpcServerConfig::default();
        // Verify it can be created
        let _ = config;
    }
}

#[cfg(not(feature = "grpc"))]
mod grpc_tests_placeholder {
    #[test]
    fn test_grpc_feature_required() {
        assert!(true, "gRPC tests require grpc feature");
    }
}
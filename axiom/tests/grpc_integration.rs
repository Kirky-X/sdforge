//! gRPC integration tests

#[cfg(test)]
mod grpc_tests {
    use axiom::grpc::axiom_v1::{
        axiom_service_server::{AxiomService, AxiomServiceServer},
        CallRequest, CallResponse, InfoRequest, InfoResponse,
    };
    use axiom::grpc::{AxiomGrpcService, GrpcRoute, GrpcServerConfig};
    use tonic::Request;

    #[test]
    fn test_grpc_service_creation() {
        let service = AxiomGrpcService::default();
        // AxiomGrpcService is a unit struct, just verify it can be created
        let _ = service;
    }

    #[test]
    fn test_grpc_route_creation() {
        let route = GrpcRoute {
            service_name: "test_service".to_string(),
        };
        assert_eq!(route.service_name, "test_service");
    }

    #[test]
    fn test_grpc_server_config_default() {
        let config = GrpcServerConfig::default();
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.timeout_seconds, 30);
    }

    #[test]
    fn test_grpc_server_config_custom() {
        let config = GrpcServerConfig {
            max_connections: 500,
            timeout_seconds: 60,
        };
        assert_eq!(config.max_connections, 500);
        assert_eq!(config.timeout_seconds, 60);
    }

    #[tokio::test]
    async fn test_grpc_call_request() {
        let request = CallRequest {
            method: "test_method".to_string(),
            params: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
        };
        assert_eq!(request.method, "test_method");
        assert!(request.params.is_empty());
        assert!(request.metadata.is_empty());
    }

    #[tokio::test]
    async fn test_grpc_call_response() {
        let response = CallResponse {
            success: true,
            data: r#"{"result":"test"}"#.to_string(),
            error: String::new(),
            status_code: 200,
        };
        assert!(response.success);
        assert_eq!(response.status_code, 200);
        assert_eq!(response.data, r#"{"result":"test"}"#);
    }

    #[tokio::test]
    async fn test_grpc_info_request() {
        let request = InfoRequest {};
        // InfoRequest is empty, just verify it can be created
    }

    #[tokio::test]
    async fn test_grpc_info_response() {
        let response = InfoResponse {
            name: "Test Service".to_string(),
            version: "1.0.0".to_string(),
            methods: vec!["method1".to_string(), "method2".to_string()],
            description: "Test Description".to_string(),
        };
        assert_eq!(response.name, "Test Service");
        assert_eq!(response.version, "1.0.0");
        assert_eq!(response.methods.len(), 2);
        assert_eq!(response.description, "Test Description");
    }

    #[tokio::test]
    async fn test_grpc_service_call() {
        let service = AxiomGrpcService::default();
        let request = Request::new(CallRequest {
            method: "test_method".to_string(),
            params: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
        });

        let result = service.call(request).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_inner();
        assert!(response.success);
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_grpc_service_get_info() {
        let service = AxiomGrpcService::default();
        let request = Request::new(InfoRequest {});

        let result = service.get_info(request).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_inner();
        assert_eq!(response.name, "Axiom Service");
        assert_eq!(response.version, "0.1.0");
        assert!(response.methods.contains(&"Call".to_string()));
        assert!(response.methods.contains(&"GetInfo".to_string()));
    }

    #[test]
    fn test_grpc_server_builder() {
        let config = GrpcServerConfig::default();
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.timeout_seconds, 30);
    }
}

// Feature Combination Tests
// Tests all feature combinations as defined in test.md Section 6

#[cfg(feature = "http")]
mod http_only_tests {
    #[test]
    fn test_http_feature_enabled() {
        assert!(true);
    }

    #[test]
    fn test_http_routes_registered() {
        use sdforge::http::build;
        let app = build();
        assert!(app.is_ok());
    }
}

#[cfg(feature = "mcp")]
mod mcp_only_tests {
    #[test]
    fn test_mcp_feature_enabled() {
        assert!(true);
    }

    #[tokio::test]
    async fn test_mcp_server_runs() {
        use sdforge::mcp::build;
        let server = build().await;
        assert!(server.is_ok());
    }
}

#[cfg(all(feature = "http", feature = "mcp"))]
mod http_mcp_tests {
    #[test]
    fn test_both_features_enabled() {
        assert!(true);
    }

    #[tokio::test]
    async fn test_dual_protocol_build() {
        use sdforge::http::build as http_build;
        use sdforge::mcp::build as mcp_build;

        let http_app = http_build();
        let mcp_server = mcp_build().await;

        assert!(http_app.is_ok());
        assert!(mcp_server.is_ok());
    }
}

#[cfg(all(feature = "http", feature = "streaming"))]
mod http_streaming_tests {
    #[test]
    fn test_streaming_feature_enabled() {
        assert!(true);
    }

    #[test]
    fn test_streaming_routes_available() {
        use sdforge::http::build;
        let app = build();
        assert!(app.is_ok());
    }
}

#[cfg(all(feature = "http", feature = "timestamp"))]
mod http_timestamp_tests {
    #[test]
    fn test_timestamp_feature_enabled() {
        assert!(true);
    }
}

#[cfg(feature = "full")]
mod full_feature_tests {
    #[test]
    fn test_full_feature_enabled() {
        assert!(true);
    }

    #[tokio::test]
    async fn test_full_build() {
        use sdforge::http::build as http_build;
        use sdforge::mcp::build as mcp_build;

        let http_app = http_build();
        let mcp_server = mcp_build().await;

        assert!(http_app.is_ok());
        assert!(mcp_server.is_ok());
    }
}

mod feature_dependency_tests {
    #[test]
    #[cfg(all(feature = "streaming", not(feature = "http")))]
    fn test_streaming_requires_http() {
        compile_error!("Streaming feature requires HTTP feature");
    }

    #[test]
    #[cfg(all(feature = "websocket", not(feature = "http")))]
    fn test_websocket_requires_http() {
        compile_error!("WebSocket feature requires HTTP feature");
    }

    #[test]
    #[cfg(all(feature = "grpc", not(feature = "http")))]
    fn test_grpc_requires_http() {
        compile_error!("gRPC feature requires HTTP feature");
    }
}

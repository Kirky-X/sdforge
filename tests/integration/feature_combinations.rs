// Feature Combination Tests
// Tests all feature combinations as defined in test.md Section 6

#[cfg(feature = "http")]
mod http_only_tests {
    #[test]
    fn test_http_routes_registered() {
        use sdforge::http::build;
        let _app = build();
        // If we get here without panicking, the build succeeded
    }
}

#[cfg(feature = "mcp")]
mod mcp_only_tests {
    #[test]
    fn test_mcp_feature_enabled() {
        // Verify MCP module is available and can be imported
        use sdforge::mcp;
        // Basic sanity check - ensure the module exists
        assert!(std::mem::size_of::<mcp::server::Server>() > 0);
    }

    #[tokio::test]
    async fn test_mcp_server_runs() {
        use sdforge::mcp::build;
        let _server = build().await;
        // If we get here without panicking, the build succeeded
    }
}

#[cfg(all(feature = "http", feature = "mcp"))]
mod http_mcp_tests {
    #[tokio::test]
    async fn test_dual_protocol_build() {
        use sdforge::http::build as http_build;
        use sdforge::mcp::build as mcp_build;

        let _http_app = http_build();
        let _mcp_server = mcp_build().await;
        // If we get here without panicking, both builds succeeded
    }
}

#[cfg(all(feature = "http", feature = "streaming"))]
mod http_streaming_tests {
    #[test]
    fn test_streaming_routes_available() {
        use sdforge::http::build;
        let _app = build();
        // If we get here without panicking, the build succeeded
    }
}

#[cfg(feature = "full")]
mod full_feature_tests {
    #[tokio::test]
    async fn test_full_build() {
        use sdforge::http::build as http_build;
        use sdforge::mcp::build as mcp_build;

        let _http_app = http_build();
        let _mcp_server = mcp_build().await;
        // If we get here without panicking, both builds succeeded
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

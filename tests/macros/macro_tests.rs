// Macro expansion tests
// Note: These tests verify macro functionality through the API

#[cfg(feature = "http")]
mod http_macro_tests {
    use sdforge::http::build;

    #[test]
    fn test_http_build() {
        let _ = build();
    }
}

#[cfg(feature = "mcp")]
mod mcp_macro_tests {
    use sdforge::mcp::build;

    #[tokio::test]
    async fn test_mcp_build() {
        let _ = build().await;
    }
}

#[cfg(all(feature = "http", feature = "mcp"))]
mod dual_macro_tests {
    use sdforge::{http::build as http_build, mcp::build as mcp_build};

    #[tokio::test]
    async fn test_dual_build() {
        let _ = http_build();
        let _ = mcp_build().await;
    }
}

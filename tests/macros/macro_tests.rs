// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
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
        let _ = build();
    }
}

#[cfg(all(feature = "http", feature = "mcp"))]
mod dual_macro_tests {
    use sdforge::{http::build as http_build, mcp::build as mcp_build};

    #[tokio::test]
    async fn test_dual_build() {
        let _ = http_build();
        let _ = mcp_build();
    }
}

#[cfg(feature = "http")]
mod service_module_prefix_tests {
    use axum::body::Body;
    use axum::http::Request;
    use sdforge::service_module;
    use tower::ServiceExt;

    #[service_module(prefix = "/auth")]
    mod auth_endpoints {
        use sdforge::forge;

        #[forge(name = "prefix_probe", version = "v1", path = "/probe", method = "GET")]
        async fn probe() -> String {
            "pong".to_string()
        }
    }

    #[test]
    fn module_prefix_const_and_helper_expose_the_prefix() {
        // `#[service_module]` emits the prefix const/helper beside the module.
        assert_eq!(MODULE_PREFIX, "/auth");
        assert_eq!(apply_prefix("/probe"), "/auth/probe");
        assert_eq!(apply_prefix("probe"), "/auth/probe");
    }

    /// `#[service_module(prefix = "/auth")]` 内的 `#[forge]` 端点注册在
    /// `/auth/api/v1/probe`（前缀拼接生效），原路径不再可用。
    #[tokio::test]
    async fn module_prefix_is_part_of_the_generated_route_path() {
        let router = sdforge::http::build();

        let prefixed = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/auth/api/v1/probe")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(prefixed.status(), 200);

        let unprefixed = router
            .oneshot(
                Request::builder()
                    .uri("/api/v1/probe")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unprefixed.status(), 404);
    }
}

#[cfg(feature = "http")]
mod cache_ttl_passthrough_tests {
    use sdforge::forge;

    #[forge(
        name = "ttl_probe",
        version = "v1",
        path = "/ttl-probe",
        method = "GET",
        cache_ttl = 60
    )]
    async fn ttl_probe() -> String {
        "ok".to_string()
    }

    #[test]
    fn cache_ttl_value_passes_through_without_clamping() {
        // 宏层与 ApiMetadata 层语义一致：用户值原样保留，不静默抬到 300
        // （历史行为曾把用户值钳制到下限 300，用户写 60 实得 300）
        assert_eq!(__axiom_metadata_ttl_probe().cache_ttl(), Some(60));
    }
}

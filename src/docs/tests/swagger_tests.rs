// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Swagger UI Router 测试。
//!
//! 对应任务：T014。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::docs::swagger_ui_router;

/// `swagger_ui_router()` 应返回有效的 `axum::Router`（编译通过 + 不 panic）。
#[test]
fn test_swagger_ui_router_returns_router() {
    let router = swagger_ui_router();
    // 编译通过即验证返回值类型为 axum::Router
    let _router: axum::Router = router;
}

/// `swagger_ui_router()` 应挂载 `/swagger-ui/` 路径，请求该路径不应返回 404。
#[tokio::test]
async fn test_swagger_ui_router_has_swagger_path() {
    let router = swagger_ui_router();
    let response = router
        .oneshot(
            Request::builder()
                .uri("/swagger-ui/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("请求应成功");

    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "/swagger-ui/ 不应返回 404"
    );
}

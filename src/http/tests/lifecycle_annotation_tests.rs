// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 端点生命周期注解（`#[forge(deprecated, sunset, successor)]`）的真实
//! 宏路径 e2e：fixture 用 `#[forge]` 宏声明（与
//! `src/http/lifecycle.rs` 中手工构造 `LifecycleMeta` 的运行时单测互补），
//! 经 `build()` 收集 inventory 注册后按真实路由分发断言响应头。
//!
//! sunset-only / successor-only 是合法组合——宏曾因裸 `deprecated`
//! （`Option<bool>`）在 `None` 时渲染零 token 产出 `deprecated: ,` 语法
//! 错误而拒绝它们，本模块锁定宏路径不再回归。

use crate::http::build;
use axum::body::Body;
use axum::http::Request;
use sdforge_macros::forge;
use tower::ServiceExt;

// ============================================================================
// 宏 fixture：sunset-only / successor-only / 全注解三条注册路径
// ============================================================================

#[forge(
    name = "lifecycle_sunset_only",
    version = "v1",
    path = "/lifecycle-sunset-only",
    method = "GET",
    description = "Sunset-only lifecycle fixture",
    sunset = "2027-01-01"
)]
async fn lifecycle_sunset_only() -> String {
    "sunsetting".to_string()
}

#[forge(
    name = "lifecycle_successor_only",
    version = "v1",
    path = "/lifecycle-successor-only",
    method = "GET",
    description = "Successor-only lifecycle fixture",
    successor = "/api/v2/lifecycle_successor"
)]
async fn lifecycle_successor_only() -> String {
    "succeeded".to_string()
}

#[forge(
    name = "lifecycle_full_annotation",
    version = "v1",
    path = "/lifecycle-full",
    method = "GET",
    description = "Full lifecycle fixture",
    deprecated,
    sunset = "2026-12-31",
    successor = "/api/v2/lifecycle_successor"
)]
async fn lifecycle_full_annotation() -> String {
    "legacy".to_string()
}

async fn get(path: &str) -> axum::http::Response<Body> {
    build()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

// ============================================================================
// sunset-only：真实宏路径下只盖 Sunset 章
// ============================================================================

#[tokio::test]
async fn sunset_only_macro_path_emits_sunset_without_deprecation() {
    let response = get("/api/v1/lifecycle-sunset-only").await;
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("sunset").unwrap(),
        "2027-01-01",
        "macro-declared sunset must reach the response headers verbatim"
    );
    assert!(
        response.headers().get("deprecation").is_none(),
        "sunset-only must NOT be stamped Deprecation (endpoint is not deprecated)"
    );
    assert!(response.headers().get("link").is_none());
}

// ============================================================================
// successor-only：真实宏路径下只盖 Link 章
// ============================================================================

#[tokio::test]
async fn successor_only_macro_path_emits_link_without_deprecation() {
    let response = get("/api/v1/lifecycle-successor-only").await;
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("link").unwrap(),
        "</api/v2/lifecycle_successor>; rel=\"successor-version\"",
        "macro-declared successor must render the successor-version link"
    );
    assert!(response.headers().get("deprecation").is_none());
    assert!(response.headers().get("sunset").is_none());
}

// ============================================================================
// 全注解：三头齐发（deprecated 裸布尔与 sunset/successor 混用的展开回归）
// ============================================================================

#[tokio::test]
async fn full_annotation_macro_path_emits_all_three_headers() {
    let response = get("/api/v1/lifecycle-full").await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("deprecation").unwrap(), "true");
    assert_eq!(response.headers().get("sunset").unwrap(), "2026-12-31");
    assert_eq!(
        response.headers().get("link").unwrap(),
        "</api/v2/lifecycle_successor>; rel=\"successor-version\""
    );
}

// ============================================================================
// 未注解端点不得携带任何生命周期头（零注解零副作用）
// ============================================================================

#[forge(
    name = "lifecycle_unannotated",
    version = "v1",
    path = "/lifecycle-unannotated",
    method = "GET",
    description = "Unannotated control fixture"
)]
async fn lifecycle_unannotated() -> String {
    "fresh".to_string()
}

#[tokio::test]
async fn unannotated_macro_path_emits_no_lifecycle_headers() {
    let response = get("/api/v1/lifecycle-unannotated").await;
    assert_eq!(response.status(), 200);
    assert!(response.headers().get("deprecation").is_none());
    assert!(response.headers().get("sunset").is_none());
    assert!(response.headers().get("link").is_none());
}

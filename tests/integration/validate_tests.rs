// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! e2e: `#[forge(validate)]` parameter validation.
//!
//! Rules declared via `#[param(ge/le/min_length/max_length/not_blank/email)]`
//! are enforced inside the generated HTTP handler; violations return
//! 422 with field-level errors (semantic constraint violations; 400 is
//! reserved for missing/unparseable params via ApiError::InvalidInput).

#![cfg(all(feature = "http", feature = "validate"))]

use sdforge::forge;
use tower::ServiceExt;

#[forge(
    name = "validate_user",
    version = "v1",
    path = "/users",
    method = "GET",
    validate
)]
async fn search_users(
    #[param(kind = "query", ge = 1, le = 100)] page: u64,
    #[param(kind = "query", min_length = 2, max_length = 10, not_blank)] keyword: String,
) -> serde_json::Value {
    serde_json::json!({ "page": page, "keyword": keyword })
}

#[forge(
    name = "validate_subscribe",
    version = "v1",
    path = "/subscribe",
    method = "POST",
    validate
)]
async fn subscribe(#[param(kind = "body", email)] email: String) -> serde_json::Value {
    serde_json::json!({ "subscribed": email })
}

#[forge(
    name = "validate_off",
    version = "v1",
    path = "/unvalidated",
    method = "GET"
)]
async fn unvalidated(#[param(kind = "query", ge = 1)] page: u64) -> serde_json::Value {
    serde_json::json!({ "page": page })
}

async fn get(uri: &str) -> (axum::http::StatusCode, serde_json::Value) {
    let router = sdforge::http::build();
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .uri(uri)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, json)
}

#[tokio::test]
async fn valid_params_pass_through() {
    let router = sdforge::http::build();
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/users?page=3&keyword=hello")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn out_of_range_numeric_returns_422_with_field_errors() {
    let (status, json) = get("/api/v1/users?page=0&keyword=hello").await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json["code"], "UNPROCESSABLE_ENTITY");
    let errors = json["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0]["field"], "page");
    assert_eq!(errors[0]["rule"], "ge");
}

#[tokio::test]
async fn string_rules_report_multiple_field_errors() {
    // keyword "a" violates min_length(2); page 999 violates le(100).
    let (status, json) = get("/api/v1/users?page=999&keyword=a").await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    let errors = json["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 2, "both violations must be reported");
    let rules: Vec<&str> = errors.iter().map(|e| e["rule"].as_str().unwrap()).collect();
    assert!(rules.contains(&"le"));
    assert!(rules.contains(&"min_length"));
}

#[tokio::test]
async fn blank_keyword_fails_not_blank_rule() {
    // blank keyword (URL-encoded space) fails not_blank; empty string fails
    // min_length too.
    let (status, json) = get("/api/v1/users?page=1&keyword=%20").await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    let errors = json["errors"].as_array().unwrap();
    assert!(
        errors
            .iter()
            .any(|e| e["rule"] == "not_blank" && e["field"] == "keyword"),
        "expected not_blank failure, got: {json}"
    );
}

#[tokio::test]
async fn body_email_rule_enforced() {
    let router = sdforge::http::build();

    // Invalid email -> 422 with field error.
    let resp = router
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/subscribe")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(r#""not-an-email""#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["errors"][0]["field"], "email");
    assert_eq!(json["errors"][0]["rule"], "email");

    // Valid email -> 200.
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/subscribe")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(r#""user@example.com""#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn rules_without_validate_flag_are_not_enforced() {
    // `unvalidated` declares ge = 1 but no `validate` flag: page=0 passes.
    let (status, json) = get("/api/v1/unvalidated?page=0").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(json["page"], 0);
}

// ============================================================================
// gRPC 对等 —— validate 规则贯通非 HTTP 路径
// ============================================================================

#[forge(
    name = "validate_grpc_bound",
    version = "v1",
    path = "/grpc-validated",
    method = "GET",
    grpc_method = "validate_rpc",
    validate
)]
async fn grpc_validated(
    #[param(kind = "query", ge = 1, le = 100)] size: u64,
) -> Result<serde_json::Value, sdforge::core::ApiError> {
    Ok(serde_json::json!({ "size": size }))
}

#[cfg(feature = "grpc")]
mod grpc_parity {
    use sdforge::forge;
    use sdforge::grpc::SdForgeGrpcService;
    use sdforge::grpc::sdforge_v1::CallRequest;
    use sdforge::grpc::sdforge_v1::sd_forge_service_server::SdForgeService;
    use std::collections::HashMap;

    fn call_request(size: &str) -> tonic::Request<CallRequest> {
        let mut parameters = HashMap::new();
        parameters.insert("size".to_string(), size.to_string());
        tonic::Request::new(CallRequest {
            method: "validate_rpc".to_string(),
            parameters,
            data: String::new(),
        })
    }

    /// 越界参数经 gRPC 必须被拒（Red：当前 gRPC 路径无校验，999 直通）。
    #[tokio::test]
    async fn out_of_range_param_rejected_over_grpc() {
        let service = SdForgeGrpcService::default();
        let err = service.call(call_request("999")).await.unwrap_err();
        // ValidationError → invalid_argument（经统一映射表）
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        let details: serde_json::Value = serde_json::from_slice(err.details()).unwrap();
        assert_eq!(details["code"], "UNPROCESSABLE_ENTITY");
        assert_eq!(details["field"], "size");
    }

    /// 合法值经 gRPC 直通不受影响。
    #[tokio::test]
    async fn in_range_param_passes_over_grpc() {
        let service = SdForgeGrpcService::default();
        let resp = service.call(call_request("50")).await.unwrap().into_inner();
        assert!(resp.success);
        assert!(resp.data.contains("50"));
    }

    // ---- 其余五种规则各一个 gRPC 侧用例（le 见上） ----

    /// ge 规则：低于下限被拒。
    #[tokio::test]
    async fn grpc_ge_rule_rejects_below_minimum() {
        let service = SdForgeGrpcService::default();
        let err = service.call(call_request("0")).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        let details: serde_json::Value = serde_json::from_slice(err.details()).unwrap();
        assert_eq!(details["code"], "UNPROCESSABLE_ENTITY");
        assert_eq!(details["field"], "size");
    }

    // ---- 多参数 handler：min_length/max_length/not_blank/email ----

    #[forge(
        name = "validate_grpc_rules",
        version = "v1",
        path = "/grpc-rules",
        method = "POST",
        grpc_method = "validate_rules_rpc",
        validate
    )]
    async fn grpc_rules(
        #[param(kind = "query", min_length = 2, max_length = 8)] name: String,
        #[param(kind = "query", not_blank)] note: String,
        #[param(kind = "body", email)] mail: String,
    ) -> Result<serde_json::Value, sdforge::core::ApiError> {
        Ok(serde_json::json!({ "name": name, "note": note, "mail": mail }))
    }

    /// name/note 走 parameters（query 对应），mail 走 data（body_param 注入）。
    fn rules_request(name: &str, note: &str, mail: &str) -> tonic::Request<CallRequest> {
        let mut parameters = HashMap::new();
        parameters.insert("name".to_string(), name.to_string());
        parameters.insert("note".to_string(), note.to_string());
        tonic::Request::new(CallRequest {
            method: "validate_rules_rpc".to_string(),
            parameters,
            data: mail.to_string(),
        })
    }

    #[tokio::test]
    async fn grpc_min_length_rule_rejects_short_value() {
        let service = SdForgeGrpcService::default();
        let err = service
            .call(rules_request("a", "ok", "a@b.com"))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        let details: serde_json::Value = serde_json::from_slice(err.details()).unwrap();
        assert_eq!(details["field"], "name");
    }

    #[tokio::test]
    async fn grpc_max_length_rule_rejects_long_value() {
        let service = SdForgeGrpcService::default();
        let err = service
            .call(rules_request("abcdefghi", "ok", "a@b.com"))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        let details: serde_json::Value = serde_json::from_slice(err.details()).unwrap();
        assert_eq!(details["field"], "name");
    }

    #[tokio::test]
    async fn grpc_not_blank_rule_rejects_blank_value() {
        let service = SdForgeGrpcService::default();
        let err = service
            .call(rules_request("abc", "  ", "a@b.com"))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        let details: serde_json::Value = serde_json::from_slice(err.details()).unwrap();
        assert_eq!(details["field"], "note");
    }

    #[tokio::test]
    async fn grpc_email_rule_rejects_invalid_address() {
        let service = SdForgeGrpcService::default();
        let err = service
            .call(rules_request("abc", "ok", "not-an-email"))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        let details: serde_json::Value = serde_json::from_slice(err.details()).unwrap();
        assert_eq!(details["field"], "mail");
    }

    #[tokio::test]
    async fn grpc_rules_all_valid_pass_through() {
        let service = SdForgeGrpcService::default();
        let resp = service
            .call(rules_request("abc", "ok", "a@b.com"))
            .await
            .unwrap()
            .into_inner();
        assert!(resp.success);
    }
}

// ============================================================================
// `#[param(kind = "query", flatten)]` —— struct 查询参数扁平直提
//
// serde_urlencoded 只支持扁平 key=value：struct 类型 query 参数默认经
// 信封 `Query<__ForgeQueryParams>`（字段嵌套 struct）提取必然 400。
// `flatten` 显式 opt-in 后该参数改由独立的 `Query<ParamTy>` 直提
// （struct 自身扁平），与信封提取的标量 query 参数互不冲突。
// ============================================================================

#[derive(Debug, serde::Deserialize)]
struct SearchFilters {
    budget: u64,
    keyword: String,
}

/// 未加 flatten 的 struct query 参数：信封嵌套反序列化失败 → 400（行为基线守卫）。
#[forge(
    name = "query_flatten_unopted",
    version = "v1",
    path = "/flat-unopted",
    method = "GET"
)]
async fn query_flatten_unopted(
    #[param(kind = "query")] filters: SearchFilters,
) -> serde_json::Value {
    serde_json::json!({ "budget": filters.budget, "keyword": filters.keyword })
}

/// flatten 直提：struct 自身字段从 query string 扁平反序列化。
#[forge(
    name = "query_flatten_opted",
    version = "v1",
    path = "/flat-opted",
    method = "GET"
)]
async fn query_flatten_opted(
    #[param(kind = "query", flatten)] filters: SearchFilters,
) -> serde_json::Value {
    serde_json::json!({ "budget": filters.budget, "keyword": filters.keyword })
}

/// flatten 与标量 query 参数（信封提取）共存。
#[forge(
    name = "query_flatten_mixed",
    version = "v1",
    path = "/flat-mixed",
    method = "GET"
)]
async fn query_flatten_mixed(
    #[param(kind = "query", ge = 1)] page: u64,
    #[param(kind = "query", flatten)] filters: SearchFilters,
) -> serde_json::Value {
    serde_json::json!({
        "page": page,
        "budget": filters.budget,
        "keyword": filters.keyword,
    })
}

/// 多个 flatten 参数各自独立提取槽。
#[derive(Debug, serde::Deserialize)]
struct PageFilter {
    page: u64,
}

#[derive(Debug, serde::Deserialize)]
struct TextFilter {
    keyword: String,
}

#[forge(
    name = "query_flatten_two",
    version = "v1",
    path = "/flat-two",
    method = "GET"
)]
async fn query_flatten_two(
    #[param(kind = "query", flatten)] page: PageFilter,
    #[param(kind = "query", flatten)] text: TextFilter,
) -> serde_json::Value {
    serde_json::json!({ "page": page.page, "keyword": text.keyword })
}

#[tokio::test]
async fn struct_query_param_without_flatten_returns_400() {
    // 信封嵌套 struct 无法被 serde_urlencoded 反序列化——flatten 前的
    // 固有缺陷在此钉住：不带 flatten 时保持 400（fail-closed）。
    let (status, _) = get("/api/v1/flat-unopted?budget=50&keyword=hello").await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn flatten_query_param_extracts_flat_fields() {
    let (status, json) = get("/api/v1/flat-opted?budget=50&keyword=hello").await;
    assert_eq!(status, axum::http::StatusCode::OK, "body: {json}");
    assert_eq!(json["budget"], 50);
    assert_eq!(json["keyword"], "hello");
}

#[tokio::test]
async fn flatten_query_param_missing_required_field_returns_400() {
    // flatten 直提保留 serde 语义：缺必填字段仍 400（不得 fail-open）。
    let (status, _) = get("/api/v1/flat-opted?budget=50").await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn flatten_coexists_with_scalar_query_params() {
    let (status, json) = get("/api/v1/flat-mixed?page=2&budget=50&keyword=hello").await;
    assert_eq!(status, axum::http::StatusCode::OK, "body: {json}");
    assert_eq!(json["page"], 2);
    assert_eq!(json["budget"], 50);
    assert_eq!(json["keyword"], "hello");
}

#[tokio::test]
async fn multiple_flatten_query_params_extract_independently() {
    let (status, json) = get("/api/v1/flat-two?page=3&keyword=hello").await;
    assert_eq!(status, axum::http::StatusCode::OK, "body: {json}");
    assert_eq!(json["page"], 3);
    assert_eq!(json["keyword"], "hello");
}

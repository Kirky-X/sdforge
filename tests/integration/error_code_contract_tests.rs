// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 跨协议错误码行为契约 e2e：同一 `ApiError` 在 HTTP 与 gRPC 两条 wire 上
//! 的状态码与机器码必须经 `error::unified` 契约表对齐（README 路线图
//! 「错误码行为契约统一」的留证测试）。
//!
//! Feature requirement:
//! `cargo test --features http,grpc --test error_code_contract_tests`.

#![cfg(all(feature = "http", feature = "grpc"))]

use sdforge::core::ApiError;
use sdforge::error::unified;
use sdforge::forge;
use sdforge::grpc::SdForgeGrpcService;
use sdforge::grpc::sdforge_v1::CallRequest;
use sdforge::grpc::sdforge_v1::sd_forge_service_server::SdForgeService;
use tower::ServiceExt;

// ============================================================================
// 夹具：三个端点各直返一种 ApiError，同一端点同时暴露 HTTP 与 gRPC 通道
// ============================================================================

#[forge(
    name = "contract_validation",
    version = "v1",
    path = "/contract/validation",
    method = "GET",
    grpc_method = "contract_validation_rpc"
)]
async fn contract_validation() -> Result<serde_json::Value, ApiError> {
    Err(ApiError::ValidationError {
        field: "email".to_string(),
        constraint: "format".to_string(),
    })
}

#[forge(
    name = "contract_invalid_input",
    version = "v1",
    path = "/contract/invalid-input",
    method = "GET",
    grpc_method = "contract_invalid_input_rpc"
)]
async fn contract_invalid_input() -> Result<serde_json::Value, ApiError> {
    Err(ApiError::InvalidInput {
        message: "malformed".to_string(),
        field: Some("age".to_string()),
        value: None,
    })
}

#[forge(
    name = "contract_not_found",
    version = "v1",
    path = "/contract/not-found",
    method = "GET",
    grpc_method = "contract_not_found_rpc"
)]
async fn contract_not_found() -> Result<serde_json::Value, ApiError> {
    Err(ApiError::NotFound {
        resource: "user".to_string(),
        resource_id: None,
    })
}

// ============================================================================
// 契约用例：HTTP wire 与 gRPC wire 联合断言
// ============================================================================

/// 同一错误经两条协议通道返回时：HTTP 状态码经 `grpc_code_for_http_status`
/// 契约表恰得 gRPC 状态码，且 UnifiedError 载荷 `code` 两侧逐字节一致。
/// 破坏任一协议侧的映射（或两侧对齐）即红灯。
#[tokio::test]
async fn same_error_agrees_across_http_and_grpc_wires() {
    let cases: Vec<(&str, &str, u16, &str, tonic::Code)> = vec![
        (
            "/api/v1/contract/validation",
            "contract_validation_rpc",
            422,
            "UNPROCESSABLE_ENTITY",
            tonic::Code::InvalidArgument,
        ),
        (
            "/api/v1/contract/invalid-input",
            "contract_invalid_input_rpc",
            400,
            "BAD_REQUEST",
            tonic::Code::InvalidArgument,
        ),
        (
            "/api/v1/contract/not-found",
            "contract_not_found_rpc",
            404,
            "NOT_FOUND",
            tonic::Code::NotFound,
        ),
    ];

    // 两条 wire 的服务端各构造一次，循环内复用（oneshot 消耗所有权故 clone）。
    let http_app = sdforge::http::build();
    let grpc_service = SdForgeGrpcService::default();

    for (path, grpc_method, expect_status, expect_code, expect_grpc) in cases {
        // HTTP wire：状态码 + UnifiedError 载荷 code。
        let resp = http_app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(path)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status().as_u16(),
            expect_status,
            "HTTP status drift for {path}"
        );
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let http_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            http_json["code"], expect_code,
            "HTTP payload code drift for {path}"
        );

        // gRPC wire：状态码 + Status::details 中的 UnifiedError code。
        let err = grpc_service
            .call(tonic::Request::new(CallRequest {
                method: grpc_method.to_string(),
                parameters: std::collections::HashMap::new(),
                data: String::new(),
            }))
            .await
            .unwrap_err();
        assert_eq!(
            err.code(),
            expect_grpc,
            "gRPC status drift for {grpc_method}"
        );
        let details: serde_json::Value = serde_json::from_slice(err.details()).unwrap();
        assert_eq!(
            details["code"], expect_code,
            "details payload code drift for {grpc_method}"
        );

        // 联合对齐：两侧值经契约表互相派生一致。
        assert_eq!(
            unified::grpc_code_for_http_status(expect_status),
            err.code(),
            "cross-protocol table misalignment for {path}"
        );
        assert_eq!(http_json["code"], details["code"]);
    }
}

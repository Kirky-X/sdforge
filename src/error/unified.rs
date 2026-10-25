// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Unified error contract.
//!
//! All protocol error responses share one structure — error code, message,
//! and the ambient request `trace_id` (from the request context when the
//! `context` feature is on). This collapses the historical HTTP-400 vs
//! gRPC-422 divergence onto a single payload shape with a
//! per-protocol status mapping table.
//!
//! ```json
//! {"code":"BAD_REQUEST","message":"...","trace_id":"trace-...","field":"age"}
//! ```
//!
//! 同一状态码下可能有多种错误（429 的速率超限与配额用尽共享状态码与机器码
//! `TOO_MANY_REQUESTS`）。自由文本 `message` 是本地化文案、不构成契约，故
//! 这类错误另带机器可读的 `details`（见 [`UnifiedError::details`]）。

use serde::Serialize;

/// Unified, protocol-neutral error payload.
#[derive(Debug, Clone, Serialize)]
pub struct UnifiedError {
    /// Stable machine-readable error code (e.g. `BAD_REQUEST`).
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Ambient trace id from the request context, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Offending field for validation-type errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Machine-readable details for errors that share one status code and one
    /// machine code — e.g. 429 `RateLimitExceeded` vs `QuotaExhausted`.
    ///
    /// `details.type` 取 `ApiError` 变体名（与 `ApiError` 自身的 serde 判
    /// 别名一致），不本地化；只有需要区分的变体才带此字段，其余错误载荷
    /// 形状不变。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl UnifiedError {
    /// Build an error payload with the ambient trace id attached.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            trace_id: current_trace_id(),
            field: None,
            details: None,
        }
    }

    /// Attach an offending field.
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// Render as a JSON value.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Ambient trace id from the request context, when the `context`
/// feature is enabled and a context is installed.
pub fn current_trace_id() -> Option<String> {
    #[cfg(feature = "context")]
    {
        crate::context::current().map(|ctx| ctx.trace_id().to_string())
    }
    #[cfg(not(feature = "context"))]
    {
        None
    }
}

/// Default error-code mapping per HTTP status (status → code).
///
/// This is the **single error-code vocabulary** of the framework: the
/// machine-readable code of an [`crate::core::ApiError`] is derived from it
/// (see [`crate::core::ApiError::code`]), and every protocol surface — HTTP
/// body, gRPC `Status::details`, `ServiceError`, MCP output — uses that one
/// code for the same error.
pub fn code_for_http_status(status: u16) -> &'static str {
    match status {
        400 => "BAD_REQUEST",
        401 => "UNAUTHORIZED",
        403 => "FORBIDDEN",
        404 => "NOT_FOUND",
        409 => "CONFLICT",
        422 => "UNPROCESSABLE_ENTITY",
        429 => "TOO_MANY_REQUESTS",
        500 => "INTERNAL",
        503 => "UNAVAILABLE",
        _ => "ERROR",
    }
}

/// HTTP 状态的直读入口（热路径去重）：单次 `to_service_error` 构造。
///
/// 状态真值仍是 `to_service_error.http_status` —— 本函数只是避免调用方
/// 为拿一个 u16 而完整构造（含 details JSON 分配）。
pub fn http_status_for(e: &crate::core::ApiError) -> u16 {
    e.http_status()
}

/// **Single source of truth** mapping an [`crate::core::ApiError`] to its
/// `(HTTP status, machine-readable code)` pair.
///
/// Every protocol adapter (HTTP `IntoResponse`, gRPC status mapping, …)
/// must derive its wire codes from here — never from a private match table.
/// The status is [`ApiError::http_status`]; the code is [`ApiError::code`],
/// i.e. the same vocabulary the other protocol surfaces use.
pub fn mapping_for(e: &crate::core::ApiError) -> (u16, &'static str) {
    (e.http_status(), e.code())
}

impl From<&crate::core::ApiError> for UnifiedError {
    fn from(e: &crate::core::ApiError) -> Self {
        // 单次构造：status/code/message 全部取自同一个 ApiError 事实来源。
        let svc = e.to_service_error();
        let message = svc.message().to_string();
        let field = match e {
            crate::core::ApiError::InvalidInput { field: Some(f), .. }
            | crate::core::ApiError::ValidationError { field: f, .. } => Some(f.clone()),
            _ => None,
        };
        Self {
            code: e.code().to_string(),
            message,
            trace_id: current_trace_id(),
            field,
            details: shared_status_details(e),
        }
    }
}

/// 429 家族的判别细节。
///
/// 速率超限与配额用尽共享状态码 429 与机器码 `TOO_MANY_REQUESTS`——客户端
/// 需据此决定退避重试还是停止重试，但 `message` 是本地化文案、不构成契约，
/// 故另给机器可读的 `details.type`（`ApiError` 变体名）与各自的计数字段。
pub(crate) fn shared_status_details(e: &crate::core::ApiError) -> Option<serde_json::Value> {
    use crate::core::ApiError;
    match e {
        ApiError::RateLimitExceeded {
            limit,
            window_seconds,
        } => Some(serde_json::json!({
            "type": "RateLimitExceeded",
            "limit": limit,
            "window_seconds": window_seconds,
        })),
        ApiError::QuotaExhausted { used, total } => Some(serde_json::json!({
            "type": "QuotaExhausted",
            "used": used,
            "total": total,
        })),
        _ => None,
    }
}

/// **跨协议行为契约表**：HTTP 状态码 → [`tonic::Code`] 的对齐轴。
///
/// 与 [`code_for_http_status`] 覆盖同一 HTTP 状态集合，是 HTTP 侧
/// [`mapping_for`] 与 gRPC 侧 [`grpc_code_for`] 的公共契约：同一错误在两
/// 协议上的状态码必须经本表对齐（一致性测试钉死，任一侧单独漂移即红灯）。
/// 400（语法畸形）与 422（语义约束违规）在 gRPC 侧同为 `invalid_argument`
/// —— gRPC 没有与 422 对应的原生状态码，语义区分由 HTTP 状态码与
/// [`UnifiedError`] 载荷的 `code` 字段（HTTP body 与 gRPC `Status::details`
/// 共享）承载。
#[cfg(feature = "grpc")]
pub fn grpc_code_for_http_status(status: u16) -> tonic::Code {
    match status {
        400 | 422 => tonic::Code::InvalidArgument,
        401 => tonic::Code::Unauthenticated,
        403 => tonic::Code::PermissionDenied,
        404 => tonic::Code::NotFound,
        // 409 在本框架的 gRPC wire 语义即幂等在途的 already_exists
        //（grpc_impl 前置守卫），契约表与既有行为对齐。
        409 => tonic::Code::AlreadyExists,
        429 => tonic::Code::ResourceExhausted,
        500 => tonic::Code::Internal,
        503 => tonic::Code::Unavailable,
        _ => tonic::Code::Unknown,
    }
}

/// **Single source of truth** mapping an [`crate::core::ApiError`] to its
/// [`tonic::Code`] (gRPC status code). Companion to [`mapping_for`] on the
/// gRPC wire.
#[cfg(feature = "grpc")]
pub fn grpc_code_for(e: &crate::core::ApiError) -> tonic::Code {
    use crate::core::ApiError;
    match e {
        ApiError::NotFound { .. } => tonic::Code::NotFound,
        ApiError::InvalidInput { .. } | ApiError::ValidationError { .. } => {
            tonic::Code::InvalidArgument
        }
        ApiError::AuthenticationFailed { .. } => tonic::Code::Unauthenticated,
        ApiError::AccessDenied { .. } => tonic::Code::PermissionDenied,
        ApiError::RateLimitExceeded { .. } | ApiError::QuotaExhausted { .. } => {
            tonic::Code::ResourceExhausted
        }
        ApiError::ServiceUnavailable { .. } => tonic::Code::Unavailable,
        ApiError::Internal { .. } => tonic::Code::Internal,
    }
}

#[cfg(feature = "http")]
mod http_render {
    use super::UnifiedError;
    use axum::response::{IntoResponse, Response};

    /// Render a unified error as an HTTP response (payload shape shared by
    /// all protocols; status drives the code via `code_for_http_status`).
    pub fn to_response(status: axum::http::StatusCode, err: &UnifiedError) -> Response {
        (status, axum::Json(err.to_json())).into_response()
    }
}

#[cfg(feature = "http")]
pub use http_render::to_response as render_http;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ApiError;
    use crate::error::SdForgeError;

    /// http_status_for / mapping_for / to_service_error 三方一致。
    #[test]
    fn http_status_for_consistent_with_service_error() {
        let variants = vec![
            ApiError::NotFound {
                resource: "x".into(),
                resource_id: None,
            },
            ApiError::InvalidInput {
                message: "x".into(),
                field: None,
                value: None,
            },
            ApiError::ValidationError {
                field: "f".into(),
                constraint: "c".into(),
            },
            ApiError::AuthenticationFailed { reason: "x".into() },
            ApiError::AccessDenied {
                permission: "p".into(),
                user_id: None,
            },
            ApiError::RateLimitExceeded {
                limit: 1,
                window_seconds: 1,
            },
            ApiError::QuotaExhausted { used: 1, total: 2 },
            ApiError::ServiceUnavailable {
                service: "s".into(),
                retry_after: None,
                source: None,
            },
            ApiError::Internal {
                message: "x".into(),
                error_id: "e".into(),
                source: None,
                context: None,
            },
        ];
        for e in variants {
            let direct = e.to_service_error().http_status();
            assert_eq!(http_status_for(&e), direct);
            assert_eq!(mapping_for(&e).0, direct);
        }
    }

    /// 统一映射表（单一事实来源）— 全变体断言。
    /// HTTP 状态与既有 `to_service_error` 的 http_status 一致；
    /// code 字符串走 `code_for_http_status` 派生。
    #[test]
    fn mapping_for_covers_all_variants() {
        let cases: Vec<(ApiError, u16, &str)> = vec![
            (
                ApiError::NotFound {
                    resource: "User".into(),
                    resource_id: None,
                },
                404,
                "NOT_FOUND",
            ),
            (
                ApiError::InvalidInput {
                    message: "x".into(),
                    field: Some("age".into()),
                    value: None,
                },
                400,
                "BAD_REQUEST",
            ),
            (
                ApiError::ValidationError {
                    field: "email".into(),
                    constraint: "format".into(),
                },
                422,
                "UNPROCESSABLE_ENTITY",
            ),
            (
                ApiError::AuthenticationFailed { reason: "x".into() },
                401,
                "UNAUTHORIZED",
            ),
            (
                ApiError::AccessDenied {
                    permission: "read".into(),
                    user_id: None,
                },
                403,
                "FORBIDDEN",
            ),
            (
                ApiError::RateLimitExceeded {
                    limit: 10,
                    window_seconds: 60,
                },
                429,
                "TOO_MANY_REQUESTS",
            ),
            (
                ApiError::QuotaExhausted { used: 1, total: 2 },
                429,
                "TOO_MANY_REQUESTS",
            ),
            (
                ApiError::ServiceUnavailable {
                    service: "db".into(),
                    retry_after: None,
                    source: None,
                },
                503,
                "UNAVAILABLE",
            ),
            (
                ApiError::Internal {
                    message: "boom".into(),
                    error_id: "e1".into(),
                    source: None,
                    context: None,
                },
                500,
                "INTERNAL",
            ),
        ];
        for (e, status, code) in cases {
            let (got_status, got_code) = mapping_for(&e);
            assert_eq!(got_status, status, "status mismatch for {e:?}");
            assert_eq!(got_code, code, "code mismatch for {e:?}");
        }
    }

    /// From<&ApiError> 字段提取：ValidationError/InvalidInput 带 field；
    /// Internal 的 message 必须脱敏（不携带构造期原始内部消息）。
    #[test]
    fn unified_error_from_api_error_extracts_fields() {
        let validation = ApiError::ValidationError {
            field: "email".into(),
            constraint: "format".into(),
        };
        let u = UnifiedError::from(&validation);
        assert_eq!(u.field.as_deref(), Some("email"));
        assert_eq!(u.code, "UNPROCESSABLE_ENTITY");

        let invalid = ApiError::InvalidInput {
            message: "bad".into(),
            field: Some("age".into()),
            value: None,
        };
        let u = UnifiedError::from(&invalid);
        assert_eq!(u.field.as_deref(), Some("age"));
        assert_eq!(u.code, "BAD_REQUEST");

        let internal = ApiError::Internal {
            message: "secret-host-db-path".into(),
            error_id: "e1".into(),
            source: None,
            context: None,
        };
        let u = UnifiedError::from(&internal);
        assert_eq!(u.code, "INTERNAL");
        assert!(u.field.is_none());
        assert!(
            !u.message.contains("secret-host-db-path"),
            "internal message must be sanitized, got: {}",
            u.message
        );
    }

    /// 429 家族共享状态码与机器码：载荷须带机器可读的判别细节，客户端
    /// 才能区分「退避重试」（速率超限）与「停止重试」（配额用尽）。
    #[test]
    fn rate_limit_family_carries_machine_readable_discriminator() {
        let rate = ApiError::RateLimitExceeded {
            limit: 10,
            window_seconds: 60,
        };
        let quota = ApiError::QuotaExhausted { used: 1, total: 2 };

        // 状态码与机器码仍同源同值——区分位不另立错误码。
        assert_eq!(mapping_for(&rate), mapping_for(&quota));

        let u = UnifiedError::from(&rate);
        let details = u.details.as_ref().expect("rate limit details");
        assert_eq!(details["type"], "RateLimitExceeded");
        assert_eq!(details["limit"], 10);
        assert_eq!(details["window_seconds"], 60);

        let u = UnifiedError::from(&quota);
        let details = u.details.as_ref().expect("quota details");
        assert_eq!(details["type"], "QuotaExhausted");
        assert_eq!(details["used"], 1);
        assert_eq!(details["total"], 2);

        // 判别位随载荷下发，客户端不必解析本地化文案。
        let json = u.to_json();
        assert_eq!(json["code"], "TOO_MANY_REQUESTS");
        assert_eq!(json["details"]["type"], "QuotaExhausted");
    }

    /// 不需要区分的错误不带 `details` 字段：载荷形状保持不变。
    #[test]
    fn errors_without_shared_status_carry_no_details() {
        let not_found = ApiError::NotFound {
            resource: "x".into(),
            resource_id: None,
        };
        let u = UnifiedError::from(&not_found);
        assert!(u.details.is_none());
        assert!(u.to_json().get("details").is_none());
    }

    #[cfg(feature = "grpc")]
    #[test]
    fn grpc_code_for_covers_all_variants() {
        use tonic::Code;
        let cases: Vec<(ApiError, Code)> = vec![
            (
                ApiError::NotFound {
                    resource: "x".into(),
                    resource_id: None,
                },
                Code::NotFound,
            ),
            (
                ApiError::InvalidInput {
                    message: "x".into(),
                    field: None,
                    value: None,
                },
                Code::InvalidArgument,
            ),
            (
                ApiError::ValidationError {
                    field: "f".into(),
                    constraint: "c".into(),
                },
                Code::InvalidArgument,
            ),
            (
                ApiError::AuthenticationFailed { reason: "x".into() },
                Code::Unauthenticated,
            ),
            (
                ApiError::AccessDenied {
                    permission: "p".into(),
                    user_id: None,
                },
                Code::PermissionDenied,
            ),
            (
                ApiError::RateLimitExceeded {
                    limit: 1,
                    window_seconds: 1,
                },
                Code::ResourceExhausted,
            ),
            (
                ApiError::QuotaExhausted { used: 1, total: 2 },
                Code::ResourceExhausted,
            ),
            (
                ApiError::ServiceUnavailable {
                    service: "s".into(),
                    retry_after: None,
                    source: None,
                },
                Code::Unavailable,
            ),
            (
                ApiError::Internal {
                    message: "x".into(),
                    error_id: "e".into(),
                    source: None,
                    context: None,
                },
                Code::Internal,
            ),
        ];
        for (e, code) in cases {
            assert_eq!(grpc_code_for(&e), code, "grpc code mismatch for {e:?}");
        }
    }

    // 跨协议错误码行为契约（HTTP 状态 ↔ gRPC 状态码对齐）

    /// ApiError 全变体样本（每变体一个代表值），供全集遍历测试复用。
    #[cfg(feature = "grpc")]
    fn every_variant() -> Vec<ApiError> {
        vec![
            ApiError::NotFound {
                resource: "x".into(),
                resource_id: None,
            },
            ApiError::InvalidInput {
                message: "x".into(),
                field: None,
                value: None,
            },
            ApiError::ValidationError {
                field: "f".into(),
                constraint: "c".into(),
            },
            ApiError::AuthenticationFailed { reason: "x".into() },
            ApiError::AccessDenied {
                permission: "p".into(),
                user_id: None,
            },
            ApiError::RateLimitExceeded {
                limit: 1,
                window_seconds: 1,
            },
            ApiError::QuotaExhausted { used: 1, total: 2 },
            ApiError::ServiceUnavailable {
                service: "s".into(),
                retry_after: None,
                source: None,
            },
            ApiError::Internal {
                message: "x".into(),
                error_id: "e".into(),
                source: None,
                context: None,
            },
        ]
    }

    /// 跨协议契约表逐行钉死：`code_for_http_status` 覆盖的每个 HTTP 状态
    /// 在 gRPC 侧都有唯一对齐码（400/422 同落 invalid_argument，语义区分
    /// 由 HTTP 状态与载荷 code 承载）；未登记状态落 Unknown 兜底。
    #[cfg(feature = "grpc")]
    #[test]
    fn grpc_code_for_http_status_pins_cross_protocol_table() {
        use tonic::Code;
        let cases: Vec<(u16, Code)> = vec![
            (400, Code::InvalidArgument),
            (401, Code::Unauthenticated),
            (403, Code::PermissionDenied),
            (404, Code::NotFound),
            (409, Code::AlreadyExists),
            (422, Code::InvalidArgument),
            (429, Code::ResourceExhausted),
            (500, Code::Internal),
            (503, Code::Unavailable),
        ];
        for (status, code) in cases {
            assert_eq!(
                grpc_code_for_http_status(status),
                code,
                "grpc contract mismatch for HTTP {status}"
            );
        }
        assert_eq!(grpc_code_for_http_status(418), Code::Unknown);
    }

    /// 「同行集」机器约束：`code_for_http_status` 与 `grpc_code_for_http_status`
    /// 两张契约表必须覆盖恰好相同的 HTTP 状态键集——0..=1000 全枚举下，一侧
    /// 有专属映射而另一侧落兜底（死键/单侧幽灵键）即红灯。键集无需第三份清单
    /// 维护，由两侧 match 表经本断言互相钉出。
    #[cfg(feature = "grpc")]
    #[test]
    fn contract_tables_share_identical_key_sets() {
        for status in 0..=1000u16 {
            assert_eq!(
                code_for_http_status(status) != "ERROR",
                grpc_code_for_http_status(status) != tonic::Code::Unknown,
                "contract key-set drift at HTTP {status}"
            );
        }
    }

    /// 跨协议 JOIN 不变量：错误码全集上，gRPC 码必须恰等于 HTTP 状态经契约
    /// 表（`grpc_code_for_http_status`）的派生值。HTTP 侧（`mapping_for`）
    /// 或 gRPC 侧（`grpc_code_for`）任何一侧单独漂移都会在此红灯——这是
    /// 「两侧映射不得各改各的」的机器钉死。
    #[cfg(feature = "grpc")]
    #[test]
    fn grpc_code_agrees_with_http_status_contract_for_every_variant() {
        for e in every_variant() {
            let (status, _) = mapping_for(&e);
            assert_eq!(
                grpc_code_for(&e),
                grpc_code_for_http_status(status),
                "cross-protocol drift for {e:?}"
            );
        }
    }

    /// 机器码字符串与 gRPC 码成对出现在契约表的同一行：载荷 `code`（HTTP
    /// body 与 gRPC details 共享）与 gRPC 状态码的对应关系在全集上可预测。
    #[cfg(feature = "grpc")]
    #[test]
    fn wire_code_pairs_are_stable_across_protocols() {
        let cases: Vec<(ApiError, u16, &str, tonic::Code)> = vec![
            (
                ApiError::ValidationError {
                    field: "email".into(),
                    constraint: "format".into(),
                },
                422,
                "UNPROCESSABLE_ENTITY",
                tonic::Code::InvalidArgument,
            ),
            (
                ApiError::InvalidInput {
                    message: "x".into(),
                    field: None,
                    value: None,
                },
                400,
                "BAD_REQUEST",
                tonic::Code::InvalidArgument,
            ),
            (
                ApiError::NotFound {
                    resource: "x".into(),
                    resource_id: None,
                },
                404,
                "NOT_FOUND",
                tonic::Code::NotFound,
            ),
        ];
        for (e, status, code, grpc) in cases {
            assert_eq!(mapping_for(&e), (status, code));
            assert_eq!(grpc_code_for(&e), grpc);
        }
    }

    /// 单一词表不变量：同一个错误在任何协议面上都得到同一个机器码。
    ///
    /// `ApiError::code()`（HTTP body / gRPC details / ServiceError / MCP 的
    /// 共同来源）、`code_for_http_status(http_status)` 与 `to_service_error`
    /// 携带的 code 必须逐字节一致——任一侧单独改词表即红灯。
    /// `SdForgeError` 的各变体（含 `Api` 之外的鉴权/配置/内部变体）同样从本
    /// 词表取码：同一 HTTP 状态不得因错误来源不同而给出两个 wire 码。
    #[test]
    fn every_surface_reports_the_same_machine_code() {
        for e in [
            ApiError::NotFound {
                resource: "x".into(),
                resource_id: None,
            },
            ApiError::InvalidInput {
                message: "x".into(),
                field: None,
                value: None,
            },
            ApiError::ValidationError {
                field: "f".into(),
                constraint: "c".into(),
            },
            ApiError::AuthenticationFailed { reason: "x".into() },
            ApiError::AccessDenied {
                permission: "p".into(),
                user_id: None,
            },
            ApiError::RateLimitExceeded {
                limit: 1,
                window_seconds: 1,
            },
            ApiError::QuotaExhausted { used: 1, total: 2 },
            ApiError::ServiceUnavailable {
                service: "s".into(),
                retry_after: None,
                source: None,
            },
            ApiError::Internal {
                message: "x".into(),
                error_id: "e".into(),
                source: None,
                context: None,
            },
        ] {
            let code = e.code();
            assert_eq!(
                code,
                code_for_http_status(e.http_status()),
                "code/status table drift for {e:?}"
            );
            assert_eq!(
                code,
                e.to_service_error().code(),
                "ServiceError code drift for {e:?}"
            );
            assert_eq!(code, mapping_for(&e).1, "mapping_for drift for {e:?}");
            assert_eq!(
                code,
                UnifiedError::from(&e).code,
                "UnifiedError code drift for {e:?}"
            );
            let mcp = serde_json::from_str::<serde_json::Value>(&e.to_mcp_json()).unwrap();
            assert_eq!(mcp["error"]["code"], code, "MCP code drift for {e:?}");
        }

        // `SdForgeError` 的各变体（含 `Api` 之外的鉴权/配置/内部变体）同样从本
        // 词表取码：同一 HTTP 状态不得因错误来源不同而给出两个 wire 码，
        // 变体区分由 `details.type` 承载。
        let mut sdforge_cases: Vec<SdForgeError> =
            vec![SdForgeError::from(ApiError::not_found("x", None))];
        #[cfg(feature = "security")]
        sdforge_cases.push(SdForgeError::from(crate::security::AuthError::InvalidToken));
        #[cfg(feature = "security")]
        sdforge_cases.push(SdForgeError::from(crate::security::JwtError::Expired));
        #[cfg(feature = "security")]
        sdforge_cases.push(SdForgeError::from(
            crate::security::AuthConfigError::SecretTooShort { length: 10 },
        ));
        #[cfg(feature = "http")]
        sdforge_cases.push(SdForgeError::from(
            crate::config::ConfigError::FileNotFound {
                path: "/missing".to_string(),
            },
        ));
        sdforge_cases.push(SdForgeError::internal("boom"));

        for e in sdforge_cases {
            let svc = e.to_service_error();
            assert_eq!(
                svc.code(),
                code_for_http_status(svc.http_status()),
                "SdForgeError code drift for {e:?}"
            );
        }
    }

    /// 字段文档里的示例码必须出自本词表。
    ///
    /// 文档示例是协议适配器实现者选码的唯一依据——示例一旦回退到旧词表
    /// （`INVALID_INPUT` 等），实现者就会照着发一个框架任何面都不会产出的码。
    #[test]
    fn doc_example_code_comes_from_the_vocabulary() {
        let source = include_str!("unified.rs");
        let doc_line = source
            .lines()
            .find(|line| line.contains("Stable machine-readable error code (e.g."))
            .expect("the `code` field documentation must exist");
        let example = doc_line
            .split('`')
            .nth(1)
            .expect("the documentation must quote the example code");
        let vocabulary = [
            code_for_http_status(400),
            code_for_http_status(401),
            code_for_http_status(403),
            code_for_http_status(404),
            code_for_http_status(409),
            code_for_http_status(422),
            code_for_http_status(429),
            code_for_http_status(500),
            code_for_http_status(503),
        ];
        assert!(
            vocabulary.contains(&example),
            "documented example code `{example}` is not produced by the framework vocabulary"
        );
    }

    #[test]
    fn unified_error_serializes_code_and_message() {
        let mut err = UnifiedError::new("INVALID_INPUT", "bad value");
        err = err.with_field("age");
        let json = err.to_json();
        assert_eq!(json["code"], "INVALID_INPUT");
        assert_eq!(json["message"], "bad value");
        assert_eq!(json["field"], "age");
    }

    #[test]
    fn trace_id_absent_without_context() {
        // Without the `context` feature (or outside a scope) no trace id is
        // attached; the field is omitted from the payload.
        let err = UnifiedError::new("FORBIDDEN", "missing role");
        let json = err.to_json();
        if err.trace_id.is_none() {
            assert!(json.get("trace_id").is_none());
        }
    }

    #[test]
    fn http_status_codes_map_to_stable_codes() {
        assert_eq!(code_for_http_status(400), "BAD_REQUEST");
        assert_eq!(code_for_http_status(401), "UNAUTHORIZED");
        assert_eq!(code_for_http_status(403), "FORBIDDEN");
        assert_eq!(code_for_http_status(429), "TOO_MANY_REQUESTS");
        assert_eq!(code_for_http_status(500), "INTERNAL");
        assert_eq!(code_for_http_status(418), "ERROR");
    }

    #[cfg(all(feature = "http", feature = "context"))]
    #[tokio::test]
    async fn trace_id_flows_from_request_context() {
        crate::context::scope(
            crate::context::RequestContext::with_ids("r".into(), "trace-42".into()),
            async {
                let err = UnifiedError::new("FORBIDDEN", "nope");
                assert_eq!(err.trace_id.as_deref(), Some("trace-42"));
                assert_eq!(err.to_json()["trace_id"], "trace-42");
            },
        )
        .await;
    }
}

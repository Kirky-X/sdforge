// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Unified handler contract shared across protocols (gRPC, CLI, ...).
//!
//! These types let protocol dispatch layers (gRPC `Call`, CLI `dispatch`) share
//! a single function-pointer type and a common value-extraction helper, so a
//! `#[forge]` function is callable the same way regardless of protocol.
//!
//! See change `grpc-cli-runtime-dispatch` (D1) for the design rationale.

use crate::prelude::ApiError;
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Stringified parameter table.
///
/// CLI builds this from clap matches; gRPC builds it from
/// `CallRequest.parameters` + `data`. Keys are parameter names.
pub type HandlerArgs = HashMap<String, String>;

/// Optional application state.
///
/// CLI injects via `CliBuilder::with_dependencies`; gRPC injects via
/// `GrpcServerConfig.state`. Handlers declare a `State` parameter and
/// recover the concrete type via `downcast_state` (T011).
pub type HandlerState = Option<Arc<dyn Any + Send + Sync>>;

/// Handler return value — the forge function's `T: Serialize` run through
/// `serde_json::to_value`.
pub type HandlerOutput = Value;

/// Boxed, sendable future returned by a handler.
pub type HandlerFuture =
    Pin<Box<dyn Future<Output = Result<HandlerOutput, ApiError>> + Send + 'static>>;

/// Unified function-pointer type shared by CLI and gRPC registrations.
///
/// A single `HandlerFn` value is referenced by both `CliHandlerRegistration`
/// and `GrpcHandlerRegistration`, so the same `#[forge]` function is callable
/// from either protocol with zero duplication.
pub type HandlerFn = fn(HandlerArgs, HandlerState) -> HandlerFuture;

/// Smart `Value` → `String` extraction (H3 decision).
///
/// `Value::String` yields the raw string (no quotes) — friendly for CLI
/// output (`Hello, world!` not `"Hello, world!"`). Any other JSON type
/// (object/array/number/bool/null) is serialized via `Value::to_string()`.
#[must_use]
pub fn extract_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Downcast `HandlerState` to a concrete `Arc<T>`.
///
/// Returns `Err(ApiError::Internal{...})` when:
/// - `state` is `None` (no state was injected via `CliBuilder::with_dependencies` / `GrpcServerConfig.state`)
/// - downcast fails (state was injected but as a different type)
///
/// Handlers declare a `#[state] db: Arc<Db>` parameter; the macro emits
/// `let db = downcast_state::<Db>(state)?;` so the handler receives the
/// concrete `Arc<Db>` directly.
pub fn downcast_state<T: Any + Send + Sync + 'static>(
    state: HandlerState,
) -> Result<Arc<T>, ApiError> {
    let arc = state.ok_or_else(|| {
        ApiError::internal_error(
            "handler requires application state but none was injected",
            "forge.state_missing",
        )
    })?;
    arc.downcast::<T>().map_err(|_| {
        ApiError::internal_error(
            "handler state type mismatch (downcast failed)",
            "forge.state_type_mismatch",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_value_string_is_raw() {
        // String → 原始串，无引号
        assert_eq!(extract_value(&json!("hi")), "hi");
        assert_eq!(extract_value(&json!("Hello, world!")), "Hello, world!");
    }

    #[test]
    fn extract_value_object_is_json() {
        // Object → JSON 序列化
        assert_eq!(extract_value(&json!({"a": 1})), r#"{"a":1}"#);
    }

    #[test]
    fn extract_value_number_bool_null_array() {
        assert_eq!(extract_value(&json!(42)), "42");
        assert_eq!(extract_value(&json!(true)), "true");
        assert_eq!(extract_value(&json!(null)), "null");
        assert_eq!(extract_value(&json!([1, 2, 3])), "[1,2,3]");
    }

    #[test]
    fn handler_types_are_send_static() {
        // HandlerFn / HandlerFuture 必须满足 Send + 'static，方可在 tonic/clap
        // 运行时跨线程调用（R-unified-handler-001）。
        fn assert_send_static<T: Send + 'static>() {}
        assert_send_static::<HandlerFn>();
        assert_send_static::<HandlerFuture>();
    }

    #[test]
    fn handler_fn_is_constructible() {
        // 函数指针类型可构造并赋值给 HandlerFn —— 编译期验证签名正确。
        // （core 不门控，不能假设 tokio runtime，故不实际驱动 future。）
        let _f: HandlerFn = |_args, _state| Box::pin(async { Ok(Value::Null) });
    }

    // ========================================================================
    // T011: downcast_state
    // ========================================================================

    #[test]
    fn downcast_state_none_returns_internal_error() {
        // state = None → handler requires state but none injected.
        let result = downcast_state::<i32>(None);
        assert!(result.is_err());
        match result.unwrap_err() {
            ApiError::Internal { error_id, .. } => {
                assert_eq!(error_id, "forge.state_missing");
            }
            other => panic!("expected ApiError::Internal, got {other:?}"),
        }
    }

    #[test]
    fn downcast_state_wrong_type_returns_internal_error() {
        // Inject Arc<i32> but request Arc<String> → downcast fails.
        let state: HandlerState = Some(Arc::new(42_i32));
        let result = downcast_state::<String>(state);
        assert!(result.is_err());
        match result.unwrap_err() {
            ApiError::Internal { error_id, .. } => {
                assert_eq!(error_id, "forge.state_type_mismatch");
            }
            other => panic!("expected ApiError::Internal, got {other:?}"),
        }
    }

    #[test]
    fn downcast_state_correct_type_returns_arc() {
        // Inject Arc<i32> and downcast to i32 → recover the concrete Arc.
        let state: HandlerState = Some(Arc::new(42_i32));
        let arc = downcast_state::<i32>(state).expect("downcast to i32 must succeed");
        assert_eq!(*arc, 42);
    }
}

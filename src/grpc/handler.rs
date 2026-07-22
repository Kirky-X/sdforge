// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! gRPC handler registration — links a `CallRequest.method` to a forge handler.
//!
//! Emitted by the `#[forge]` macro (when `grpc_method` is set) via
//! `inventory::submit!`. At runtime, `SdForgeGrpcService::call` looks up the
//! handler by `method` and invokes it. Only forge functions that explicitly
//! declare `grpc_method` are reachable via gRPC (minimal attack surface).

use crate::core::HandlerFn;

/// Registration linking a gRPC `CallRequest.method` to a forge handler.
///
/// All fields are `Copy` (`&'static str` / `fn` pointer / `Option<&str>` /
/// `Option<u16>`) so the registration lives in read-only memory.
#[derive(Debug, Clone, Copy)]
pub struct GrpcHandlerRegistration {
    /// `CallRequest.method` match key (= the forge macro's `grpc_method` value).
    pub method: &'static str,
    /// Unified handler pointer (shared with CLI via `core::HandlerFn`).
    pub handler: HandlerFn,
    /// Body parameter name, if any. gRPC injects `CallRequest.data` into this
    /// key. `None` means no Body parameter — the `data` field is rejected.
    pub body_param: Option<&'static str>,
    /// Macro-level `status` argument (e.g. `#[forge(status = 201)]`) carried
    /// into the gRPC layer so the gRPC success path can mirror the HTTP
    /// success code. Applied as the fallback when the handler's returned
    /// `ServiceResponse` does not carry its own `status_code` field —
    /// priority chain: `ServiceResponse.status_code` > `default_status` > 200.
    /// `None` means no macro `status` was declared (default 200).
    pub default_status: Option<u16>,
}

inventory::collect!(GrpcHandlerRegistration);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{HandlerArgs, HandlerFuture, HandlerState};
    use serde_json::Value;

    fn assert_copy_clone<T: Copy + Clone>() {}

    fn dummy_handler(_args: HandlerArgs, _state: HandlerState) -> HandlerFuture {
        Box::pin(async { Ok(Value::Null) })
    }

    #[test]
    fn grpc_handler_registration_is_copy() {
        // 结构体必须 Copy/Clone（inventory 项按值遍历）
        assert_copy_clone::<GrpcHandlerRegistration>();
    }

    inventory::submit! {
        GrpcHandlerRegistration {
            method: "test_probe",
            handler: dummy_handler,
            body_param: None,
            default_status: None,
        }
    }

    #[test]
    fn grpc_handler_registration_collected() {
        // inventory 收集到本模块 submit 的 test_probe
        let count = inventory::iter::<GrpcHandlerRegistration>().count();
        assert!(count >= 1, "GrpcHandlerRegistration inventory empty");
        let names: Vec<_> = inventory::iter::<GrpcHandlerRegistration>()
            .map(|r| r.method)
            .collect();
        assert!(
            names.contains(&"test_probe"),
            "test_probe missing in {names:?}"
        );
    }
}

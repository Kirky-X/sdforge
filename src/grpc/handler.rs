// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! gRPC handler registration — links a `CallRequest.method` to a forge handler.
//!
//! Emitted by the `#[forge]` macro (when `grpc_method` is set) via
//! `inventory::submit!`. At runtime, `SdForgeGrpcService::call` looks up the
//! handler by `method` and invokes it. Only forge functions that explicitly
//! declare `grpc_method` are reachable via gRPC (minimal attack surface).

use crate::core::HandlerFn;

#[cfg(all(feature = "grpc", feature = "streaming"))]
use std::future::Future;
#[cfg(all(feature = "grpc", feature = "streaming"))]
use std::pin::Pin;

#[cfg(all(feature = "grpc", feature = "streaming"))]
use crate::core::{HandlerArgs, HandlerState};

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
    /// Endpoint RBAC roles from `#[forge(auth(role = "..."))]`. Empty slice
    /// = no role requirement (authentication still applies). With the
    /// `security` feature disabled, a non-empty declaration denies every
    /// request — fail-safe, mirroring HTTP `require_role`.
    pub roles: &'static [&'static str],
    /// Runtime translation key for the description
    /// (`#[forge(i18n_key = "...")]`). The gRPC wire has no per-method
    /// description output — hosts consuming the inventory directly use this
    /// with `sdforge::i18n::translate_or_fallback` (CLI/MCP parity);
    /// `None` keeps the compile-time English description.
    pub i18n_key: Option<&'static str>,
    /// Endpoint lifecycle flags (`#[forge(deprecated, sunset, successor)]`),
    /// mirrored onto the response metadata as `deprecation` / `sunset` /
    /// `successor-version` keys. Copy-expansion (not `LifecycleMeta`) keeps
    /// the inventory entry const-constructible.
    pub deprecated: bool,
    /// Sunset date/value surfaced verbatim on the `sunset` metadata key
    /// (e.g. `"2026-12-31"`); `None` when not declared.
    pub sunset: Option<&'static str>,
    /// Successor endpoint hint surfaced on the `successor-version` metadata
    /// key; `None` when not declared.
    pub successor: Option<&'static str>,
}

inventory::collect!(GrpcHandlerRegistration);

/// Server-streaming handler registration（feature = `grpc` + `streaming`）。
///
/// Links a `CallRequest.method` to a forge handler declared with
/// `#[forge(grpc_method = "...", stream = true)]`; invoked via the
/// `CallStream` RPC (`SdForgeService::call_stream`). Streaming methods live
/// in a registry separate from [`GrpcHandlerRegistration`], so the unary
/// `Call` path rejects them (`failed_precondition` → 指引改走 `CallStream`)
/// and vice versa — 一个方法只会落在两张表中的一张。
///
/// 语义差异（相对 unary）：流式路径不参与幂等重放（流式响应没有单点可
/// 缓存的响应体，重放语义不成立）——携带 `idempotency-key` metadata 的
/// `CallStream` 请求以 `failed_precondition` 显式拒绝，而非静默忽略。
#[derive(Debug, Clone, Copy)]
#[cfg(all(feature = "grpc", feature = "streaming"))]
pub struct GrpcStreamHandlerRegistration {
    /// `CallRequest.method` match key (= the forge macro's `grpc_method` value).
    pub method: &'static str,
    /// Streaming handler pointer. Resolves to a per-item stream of JSON
    /// values (or item-level error strings) consumed by `call_stream`.
    pub handler: GrpcStreamHandlerFn,
    /// Body parameter name, if any — same injection contract as the unary
    /// registration (`CallRequest.data` → this key).
    pub body_param: Option<&'static str>,
    /// Macro-level `status` argument, applied per stream item with the same
    /// priority chain as the unary path: `ServiceResponse.status_code` >
    /// `default_status` > 200.
    pub default_status: Option<u16>,
    /// Endpoint RBAC roles — same fail-safe contract as the unary path.
    pub roles: &'static [&'static str],
    /// Runtime translation key for the description — same contract as the
    /// unary registration's `i18n_key`.
    pub i18n_key: Option<&'static str>,
    /// Endpoint lifecycle flags — same response-metadata contract as the
    /// unary registration.
    pub deprecated: bool,
    /// Sunset date/value surfaced verbatim on the `sunset` metadata key;
    /// `None` when not declared.
    pub sunset: Option<&'static str>,
    /// Successor endpoint hint surfaced on the `successor-version` metadata
    /// key; `None` when not declared.
    pub successor: Option<&'static str>,
}

#[cfg(all(feature = "grpc", feature = "streaming"))]
inventory::collect!(GrpcStreamHandlerRegistration);

/// 把端点生命周期声明附加到成功响应的 metadata 上（unary `Call` 与
/// streaming `CallStream` 共用）。
///
/// 与 HTTP 的 `Deprecation` / `Sunset` / `Link: successor-version` 响应头
/// 镜像同名小写键（`deprecation` / `sunset` / `successor-version`）。
/// 非法（非可见 ASCII）值优雅跳过——声明了不可编码的值不得使 RPC 失败；
/// 三项皆未声明时整体 no-op，未注解端点无注入开销。
pub(crate) fn attach_lifecycle_metadata<T>(
    response: &mut tonic::Response<T>,
    deprecated: bool,
    sunset: Option<&str>,
    successor: Option<&str>,
) {
    if !deprecated && sunset.is_none() && successor.is_none() {
        return;
    }
    let metadata = response.metadata_mut();
    if deprecated {
        metadata.insert(
            tonic::metadata::MetadataKey::from_static("deprecation"),
            tonic::metadata::MetadataValue::from_static("true"),
        );
    }
    if let Some(sunset) = sunset
        && let Ok(value) = tonic::metadata::MetadataValue::try_from(sunset)
    {
        metadata.insert(tonic::metadata::MetadataKey::from_static("sunset"), value);
    }
    if let Some(successor) = successor
        && let Ok(value) = tonic::metadata::MetadataValue::try_from(successor)
    {
        metadata.insert(
            tonic::metadata::MetadataKey::from_static("successor-version"),
            value,
        );
    }
}

/// 单条流式项：`Ok` = handler 产出的一个 JSON 值（每项映射为一条
/// `success: true` 的 `CallResponse`），`Err` = 项级业务错误消息（映射为
/// `success: false` 的 `CallResponse`，流继续——与 SSE 错误事件对齐）。
#[cfg(all(feature = "grpc", feature = "streaming"))]
pub type GrpcStreamItem = Result<crate::core::HandlerOutput, String>;

/// 流式 handler 的产出：逐项流 + 每项状态码回退链的输入。
#[cfg(all(feature = "grpc", feature = "streaming"))]
pub struct GrpcStreamOutput {
    /// 逐项流。每项消费为一条 `CallResponse` 消息推给客户端。
    pub stream:
        std::pin::Pin<Box<dyn futures_util::Stream<Item = GrpcStreamItem> + Send + 'static>>,
}

/// Boxed, sendable future returned by a streaming handler.
#[cfg(all(feature = "grpc", feature = "streaming"))]
pub type GrpcStreamHandlerFuture =
    Pin<Box<dyn Future<Output = Result<GrpcStreamOutput, crate::core::ApiError>> + Send + 'static>>;

/// Unified function-pointer type for server-streaming gRPC registrations.
#[cfg(all(feature = "grpc", feature = "streaming"))]
pub type GrpcStreamHandlerFn = fn(HandlerArgs, HandlerState) -> GrpcStreamHandlerFuture;

/// 把 [`crate::streaming::StreamResponse`] 逐项序列化为流式 gRPC 产出
/// （宏为 `#[forge(grpc_method, stream = true)]` 生成的 handler 闭包消费）。
///
/// 每项经 `serde_json::to_value` 转为 [`crate::core::HandlerOutput`]；
/// 序列化失败的项降级为项级错误消息（流继续，不终止）——与 unary 路径
/// 的 `extract_value` 单源语义保持一致。
///
/// # 生产者生命周期边界（显式契约）
///
/// 用户 handler 内 `tokio::spawn` 的生产者任务 panic 或中止时，channel
/// 发送端 drop、流以**正常耗尽**收尾——已产出项照常送达，但客户端无法
/// 区分完整流与截断流（对比 unary：handler panic 经 catch_unwind 映射为
/// `Status::internal`）。这是当前流式语义的一部分（契约测试
/// `producer_panic_truncates_stream_silently_by_contract` 锁定）。需要
/// fail-visible 生产者的调用方应自行持有 JoinHandle 监测，在异常退出时
/// 向 channel 注入项级错误（`Err(msg)` 项 → success:false 消息、流继续）。
/// 另：客户端断开 → 发送端 `send` 返回 Err → 生产者循环应退出（取消
/// 传播依赖此约定）。
#[cfg(all(feature = "grpc", feature = "streaming"))]
pub fn stream_output_from<T>(response: crate::streaming::StreamResponse<T>) -> GrpcStreamOutput
where
    T: serde::Serialize + Send + 'static,
{
    use futures_util::StreamExt;

    let mapped = response
        .into_stream()
        .map(|item| item.and_then(|t| serde_json::to_value(&t).map_err(|e| e.to_string())));
    GrpcStreamOutput {
        stream: Box::pin(mapped),
    }
}

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
                roles: &[],
                i18n_key: None,
                deprecated: false,
                sunset: None,
                successor: None,
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

    /// 非法（非可见 ASCII）生命周期值必须优雅跳过而非使 RPC 失败；
    /// 未声明时整体 no-op。
    #[test]
    fn attach_lifecycle_metadata_skips_invalid_values_and_no_ops_when_absent() {
        let mut response = tonic::Response::new(());
        attach_lifecycle_metadata(&mut response, true, Some("无效\n值"), None);
        let metadata = response.metadata();
        assert_eq!(metadata.get("deprecation").unwrap(), "true");
        assert!(metadata.get("sunset").is_none());
        assert!(metadata.get("successor-version").is_none());

        let mut bare = tonic::Response::new(());
        attach_lifecycle_metadata(&mut bare, false, None, None);
        assert!(bare.metadata().is_empty());
    }
}

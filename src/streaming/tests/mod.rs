// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Streaming module test suites.
//!
//! Tests are organized by responsibility:
//! - `stream_builder_tests`: `StreamResponse`, `StreamEvent`, and `create_stream_channel`
//!   construction, serialization, deserialization, channel behavior, and Debug impls
//! - `sse_tests`: `stream_to_sse` SSE conversion and `StreamResponse`'s `IntoResponse`
//!   HTTP SSE response impl

mod sse_tests;
mod stream_builder_tests;

use crate::streaming::StreamEvent;

// ============================================================================
// Shared test helpers — accessible by all sub-modules via `super::`
// ============================================================================

pub(super) fn create_test_data_event() -> StreamEvent<serde_json::Value> {
    StreamEvent::data(serde_json::json!({"test": "value"}))
}

pub(super) fn create_test_ping_event() -> StreamEvent<()> {
    StreamEvent::ping()
}

pub(super) fn create_test_error_event(msg: &str) -> StreamEvent<()> {
    StreamEvent::error(msg.to_string())
}

pub(super) fn create_test_complete_event() -> StreamEvent<()> {
    StreamEvent::complete()
}

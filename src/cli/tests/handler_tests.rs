// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `CliHandlerRegistration` (T006).
//!
//! A `CliHandlerRegistration` pairs a command name with a `fn` pointer that
//! takes a `HashMap<String, String>` of CLI-supplied arguments and returns
//! a boxed future resolving to `Result<(), ApiError>`. The
//! `#[forge]` macro (T008) emits `inventory::submit!` blocks pairing
//! each `CliCommandRegistration` with one of these handler registrations.

use crate::cli::CliHandlerRegistration;
use crate::prelude::ApiError;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

// ============================================================================
// T006: CliHandlerRegistration
// ============================================================================

/// Test handler used by `test_handler_registration_call`. Returns `Ok(())`
/// unconditionally so the test can assert the happy path. Lives at module
/// scope because `inventory::submit!` needs a `&'static` reference (and
/// `fn` pointers are inherently `const`).
fn test_handler_call_handler(
    _args: HashMap<String, String>,
) -> Pin<Box<dyn Future<Output = Result<(), ApiError>> + Send + 'static>> {
    Box::pin(async { Ok(()) })
}

// Statically submit the test handler so `inventory::iter` can find it.
inventory::submit!(CliHandlerRegistration {
    name: "test_handler_call",
    handler: test_handler_call_handler,
});

/// Verify that a registered `CliHandlerRegistration`'s `handler` is
/// callable and returns `Ok(())` on the happy path. This exercises the
/// `inventory::collect!` + `inventory::submit!` round-trip plus the
/// async future polling.
#[tokio::test]
async fn test_handler_registration_call() {
    let found = inventory::iter::<CliHandlerRegistration>()
        .find(|h| h.name == "test_handler_call")
        .expect("test_handler_call registration must be present");

    let args = HashMap::new();
    let result = (found.handler)(args).await;
    assert!(result.is_ok(), "happy-path handler must return Ok(())");
}

/// Verify that a handler can read arguments out of the supplied
/// `HashMap<String, String>` — exercises the string-typed parameter
/// channel that the macro-generated handlers will rely on.
fn echo_handler(
    args: HashMap<String, String>,
) -> Pin<Box<dyn Future<Output = Result<(), ApiError>> + Send + 'static>> {
    Box::pin(async move {
        // Simulate the macro-generated pattern: extract a value and
        // surface a missing-required-arg failure as an ApiError.
        if args.get("input").map(String::as_str) == Some("hello") {
            Ok(())
        } else {
            Err(ApiError::InvalidInput {
                message: "expected `input=hello`".into(),
                field: Some("input".into()),
                value: None,
            })
        }
    })
}

inventory::submit!(CliHandlerRegistration {
    name: "echo_handler",
    handler: echo_handler,
});

#[tokio::test]
async fn test_handler_registration_reads_args() {
    let found = inventory::iter::<CliHandlerRegistration>()
        .find(|h| h.name == "echo_handler")
        .expect("echo_handler registration must be present");

    // Happy path: input=hello.
    let mut args = HashMap::new();
    args.insert("input".to_string(), "hello".to_string());
    let result = (found.handler)(args).await;
    assert!(result.is_ok(), "echo_handler with input=hello must succeed");

    // Failure path: missing input → InvalidInput error.
    let empty = HashMap::new();
    let result = (found.handler)(empty).await;
    assert!(result.is_err(), "echo_handler without input must fail");
    match result.unwrap_err() {
        ApiError::InvalidInput { field, .. } => {
            assert_eq!(field.as_deref(), Some("input"));
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

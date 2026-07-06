// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T009: `#[service_api(cli = true)]` end-to-end integration.
//!
//! These tests decorate real functions with the `#[service_api]` macro
//! (passing `cli = true`) and assert that the macro-generated
//! `inventory::submit!` blocks are observable through
//! `inventory::iter::<CliCommandRegistration>` /
//! `inventory::iter::<CliHandlerRegistration>`.
//!
//! Unlike the trybuild pass tests in `macros/tests/trybuild/pass/`, these
//! tests run with `--features cli` so the `#[cfg(feature = "cli")]`-gated
//! items are actually compiled and linked into the test binary.

use crate::cli::{CliArgType, CliCommandRegistration, CliHandlerRegistration};
use crate::prelude::ApiError;
use sdforge_macros::service_api;

// ============================================================================
// Test fixture: a service_api function with `cli = true` and a path param.
//
// The macro should emit:
//   1. CliCommandRegistration { name: "test_cli_macro_cmd", args: [id(Path)] }
//   2. CliHandlerRegistration { name: "test_cli_macro_cmd", handler: fn }
// ============================================================================
#[service_api(
    name = "test_cli_macro_cmd",
    version = "v1",
    description = "Test CLI macro command",
    path = "/users/:id",
    cli = true
)]
async fn test_cli_macro_cmd(id: u64) -> Result<String, ApiError> {
    Ok(format!("id={}", id))
}

// ============================================================================
// Test fixture: a parameterless service_api function with `cli = true`.
// ============================================================================
#[service_api(name = "test_cli_macro_noargs", version = "v2", cli = true)]
async fn test_cli_macro_noargs() -> Result<String, ApiError> {
    Ok("ok".to_string())
}

/// Verify the macro emits a `CliCommandRegistration` for the path-param
/// fixture, with correct metadata and a single Path argument for `id`.
#[test]
fn test_service_api_with_cli_generates_command_registration() {
    let cmd = inventory::iter::<CliCommandRegistration>()
        .find(|c| c.name == "test_cli_macro_cmd")
        .expect("CliCommandRegistration for test_cli_macro_cmd must be registered");

    assert_eq!(cmd.version, "v1");
    assert_eq!(cmd.description, "Test CLI macro command");
    assert_eq!(cmd.handler_fn_name, "test_cli_macro_cmd");

    // The `id: u64` path parameter must surface as a single CliArgType::Path
    // entry with required=true.
    assert_eq!(
        cmd.args.len(),
        1,
        "expected exactly one CLI arg for test_cli_macro_cmd"
    );
    assert_eq!(cmd.args[0].name, "id");
    assert_eq!(cmd.args[0].arg_type, CliArgType::Path);
    assert!(cmd.args[0].required);
}

/// Verify the macro emits a `CliHandlerRegistration` paired with the
/// command registration. The handler pointer must be present even if we
/// don't invoke it here — handler invocation is covered by handler_tests.
#[test]
fn test_service_api_with_cli_generates_handler_registration() {
    let handler = inventory::iter::<CliHandlerRegistration>()
        .find(|h| h.name == "test_cli_macro_cmd")
        .expect("CliHandlerRegistration for test_cli_macro_cmd must be registered");
    // The handler fn pointer is non-null by construction (inventory::submit!
    // requires a valid fn). Touching the field ensures the registration
    // layout matches what the macro emits.
    let _ = handler.handler;
}

/// Verify a parameterless `#[service_api(cli = true)]` still emits both
/// registrations with an empty args slice.
#[test]
fn test_service_api_with_cli_no_args_registers() {
    let cmd = inventory::iter::<CliCommandRegistration>()
        .find(|c| c.name == "test_cli_macro_noargs")
        .expect("CliCommandRegistration for test_cli_macro_noargs must be registered");
    assert_eq!(cmd.version, "v2");
    assert!(cmd.args.is_empty(), "expected no args for noargs fixture");

    let handler = inventory::iter::<CliHandlerRegistration>()
        .find(|h| h.name == "test_cli_macro_noargs")
        .expect("CliHandlerRegistration for test_cli_macro_noargs must be registered");
    let _ = handler.handler;
}

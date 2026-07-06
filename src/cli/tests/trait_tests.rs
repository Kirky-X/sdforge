// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Trait/structure tests for CLI registration primitives.
//!
//! Tests are added incrementally as T002/T003/T004 land. This file
//! currently covers T002 (`CliArgType` + `CliArgInfo`).

use crate::cli::{CliArgInfo, CliArgType, CliCommandRegistration};

// ============================================================================
// T002: CliArgType variants + CliArgInfo field population
// ============================================================================

/// Verify the three `CliArgType` variants exist and `PartialEq` holds for
/// each distinct variant. This is the foundational assertion for T002 —
/// if any variant is missing or `PartialEq` is not derived, compilation
/// fails here.
#[test]
fn test_cli_arg_type_variants() {
    let path = CliArgType::Path;
    let body = CliArgType::Body;
    let state = CliArgType::State;

    // PartialEq must hold: each variant equals itself, differs from others.
    assert_eq!(path, CliArgType::Path);
    assert_eq!(body, CliArgType::Body);
    assert_eq!(state, CliArgType::State);

    assert_ne!(path, body);
    assert_ne!(body, state);
    assert_ne!(path, state);
}

/// Verify `CliArgInfo::new` populates every field and the values are
/// retrievable. Also exercises `Copy` semantics (assigning the struct
/// should not move it).
#[test]
fn test_cli_arg_info_new_populates_fields() {
    let arg = CliArgInfo::new(
        "user_id",
        "ID of the user to look up",
        CliArgType::Path,
        true,
        None,
    );

    assert_eq!(arg.name, "user_id");
    assert_eq!(arg.description, "ID of the user to look up");
    assert_eq!(arg.arg_type, CliArgType::Path);
    assert!(arg.required);
    assert_eq!(arg.default, None);
}

/// Verify `CliArgInfo::new` with a `default` value stores it correctly.
#[test]
fn test_cli_arg_info_new_with_default() {
    let arg = CliArgInfo::new(
        "limit",
        "Maximum number of results",
        CliArgType::Body,
        false,
        Some("10"),
    );

    assert_eq!(arg.name, "limit");
    assert_eq!(arg.arg_type, CliArgType::Body);
    assert!(!arg.required);
    assert_eq!(arg.default, Some("10"));
}

/// Verify `CliArgInfo::new` is usable in a `const` context (required by R-cli-002).
#[test]
fn test_cli_arg_info_new_is_const_fn() {
    const ARG: CliArgInfo = CliArgInfo::new("id", "Resource ID", CliArgType::Path, true, None);
    assert_eq!(ARG.name, "id");
    assert_eq!(ARG.arg_type, CliArgType::Path);
}

// ============================================================================
// T003: CliCommandRegistration field population
// ============================================================================

/// Verify `CliCommandRegistration::new` populates all fields and `args`
/// defaults to an empty slice.
#[test]
fn test_cli_command_registration_new() {
    let reg = CliCommandRegistration::new("echo", "v1", "Echo back input", "echo_handler");

    assert_eq!(reg.name, "echo");
    assert_eq!(reg.version, "v1");
    assert_eq!(reg.description, "Echo back input");
    assert_eq!(reg.handler_fn_name, "echo_handler");
    assert!(reg.args.is_empty());
}

/// Verify `CliCommandRegistration::with_args` replaces the args slice.
#[test]
fn test_cli_command_registration_with_args() {
    const ARGS: &[CliArgInfo] = &[
        CliArgInfo::new("id", "Resource ID", CliArgType::Path, true, None),
        CliArgInfo::new("limit", "Max results", CliArgType::Body, false, Some("10")),
    ];

    let reg =
        CliCommandRegistration::new("list", "v1", "List resources", "list_handler").with_args(ARGS);

    assert_eq!(reg.args.len(), 2);
    assert_eq!(reg.args[0].name, "id");
    assert_eq!(reg.args[1].name, "limit");
}

/// Verify `CliCommandRegistration::new` is usable in a `const` context.
#[test]
fn test_cli_command_registration_const_fn() {
    const REG: CliCommandRegistration =
        CliCommandRegistration::new("ping", "v1", "Health check", "ping_handler");
    assert_eq!(REG.name, "ping");
    assert_eq!(REG.handler_fn_name, "ping_handler");
}

// ============================================================================
// T004: inventory collects CliCommandRegistration submissions
// ============================================================================

// Test-only registration submitted at module load time. The static
// lifetime is required by `inventory::submit!`. Doc comments are not
// applied to macro invocations, hence the plain `//` style here.
inventory::submit!(CliCommandRegistration::new(
    "trait_test_command",
    "v1",
    "Test command for trait_tests",
    "trait_test_handler"
)
.with_args(&[CliArgInfo::new(
    "input",
    "Input value",
    CliArgType::Path,
    true,
    None,
)]));

/// Verify that `inventory::iter::<CliCommandRegistration>()` yields the
/// statically-submitted test entry. This validates that
/// `inventory::collect!(CliCommandRegistration)` is in effect.
#[test]
fn test_inventory_collects_registration() {
    let found: Vec<_> = inventory::iter::<CliCommandRegistration>()
        .filter(|r| r.name == "trait_test_command")
        .collect();

    assert_eq!(
        found.len(),
        1,
        "expected exactly one `trait_test_command` registration"
    );

    let reg = &found[0];
    assert_eq!(reg.version, "v1");
    assert_eq!(reg.description, "Test command for trait_tests");
    assert_eq!(reg.handler_fn_name, "trait_test_handler");
    assert_eq!(reg.args.len(), 1);
    assert_eq!(reg.args[0].name, "input");
    assert_eq!(reg.args[0].arg_type, CliArgType::Path);
}

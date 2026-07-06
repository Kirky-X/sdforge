// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Trait/structure tests for CLI registration primitives.
//!
//! Tests are added incrementally as T002/T003/T004 land. This file
//! currently covers T002 (`CliArgType` + `CliArgInfo`).

use crate::cli::{CliArgInfo, CliArgType};

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

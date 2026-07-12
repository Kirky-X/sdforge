// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T010: `init_all_plugins` integration for CLI inventory.
//!
//! Verifies that `init_all_plugins()` touches the CLI inventory registrations
//! (`CliCommandRegistration` / `CliHandlerRegistration`) so the linker does
//! not strip them, and that `PluginCounts` surfaces a `cli_commands` count.
//!
//! Without T010's `init_all_plugins` extension, the `inventory::submit!`
//! blocks emitted by `#[forge(cli = true)]` may be optimized away by
//! the linker because nothing in the call graph references the CLI inventory
//! iteration. This suite asserts the integration point exists and works.

use crate::cli::{CliCommandRegistration, CliHandlerRegistration};
use crate::init_all_plugins;
use crate::prelude::ApiError;
use sdforge_macros::forge;

// ============================================================================
// Fixture: a forge function with `cli = true`.
//
// If `init_all_plugins` does not touch CLI inventory, the linker may strip
// this registration and the `inventory::iter` lookups below would return
// `None`. The fixture is intentionally distinct from the ones in
// `macro_integration_tests` so this suite owns its own registration.
// ============================================================================
#[forge(name = "t010_init_cmd", version = "v1", cli = true)]
async fn t010_init_cmd() -> Result<String, ApiError> {
    Ok("ok".to_string())
}

/// Verify `init_all_plugins()` returns a `PluginCounts` that includes a
/// `cli_commands` field, and that the CLI inventory is linked after the call.
///
/// Red phase: `counts.cli_commands` does not exist yet — T010 Green must add
/// the field and the inventory-touching block inside `init_all_plugins`.
#[test]
#[serial_test::serial]
fn test_init_all_plugins_registers_cli_commands() {
    let counts = init_all_plugins();

    // T010 Green must add the `cli_commands` field to `PluginCounts`.
    let cli_count: usize = counts.cli_commands;

    // At least the `t010_init_cmd` fixture must be registered.
    assert!(
        cli_count >= 1,
        "expected at least one CLI command registered after init_all_plugins, got {}",
        cli_count
    );

    // The fixture must be findable via inventory::iter after init — this
    // proves the linker kept the registration.
    let cmd = inventory::iter::<CliCommandRegistration>()
        .find(|c| c.name == "t010_init_cmd")
        .expect("t010_init_cmd must be linked after init_all_plugins");
    assert_eq!(cmd.version, "v1");

    let handler = inventory::iter::<CliHandlerRegistration>()
        .find(|h| h.name == "t010_init_cmd")
        .expect("t010_init_cmd handler must be linked after init_all_plugins");
    // Touching the handler fn pointer ensures the registration layout
    // matches what the macro emits.
    let _ = handler.handler;
}

/// Verify `init_all_plugins()` is idempotent for CLI inventory: calling it
/// twice returns the same `cli_commands` count (OnceLock caches the
/// iteration result).
#[test]
#[serial_test::serial]
fn test_init_all_plugins_cli_count_is_idempotent() {
    let first = init_all_plugins();
    let second = init_all_plugins();
    assert_eq!(
        first.cli_commands, second.cli_commands,
        "init_all_plugins must return stable cli_commands counts across calls"
    );
}

/// T026: Verify `init_all_plugins()` keeps working when `docs` feature is
/// enabled alongside `cli`, and that `CliBuilder::build()` then surfaces both
/// the inventory-registered commands and the auto-injected `docs` subcommand.
///
/// `docs` does not register inventory entries of its own (it is a pure
/// function module); the only integration point with `init_all_plugins` is
/// that the CLI inventory touched by T010 must remain linked so that
/// `CliBuilder::build()` can iterate it. This test guards against accidental
/// regressions where enabling `docs` could shadow the CLI inventory.
#[cfg(feature = "docs")]
#[test]
#[serial_test::serial]
fn test_init_all_plugins_with_docs_feature_keeps_cli_inventory() {
    // init_all_plugins must still return a non-zero cli_commands count when
    // docs feature is enabled (the T010 fixture `t010_init_cmd` is in this
    // test binary).
    let counts = init_all_plugins();
    assert!(
        counts.cli_commands >= 1,
        "init_all_plugins must still link CLI inventory under docs feature, got {}",
        counts.cli_commands
    );

    // CliBuilder::build() must surface both inventory commands and the
    // auto-injected docs subcommand.
    use crate::cli::CliBuilder;
    let cmd = CliBuilder::new().build();
    assert!(
        cmd.find_subcommand("docs").is_some(),
        "CliBuilder::build() must include `docs` subcommand when docs feature is enabled"
    );
}

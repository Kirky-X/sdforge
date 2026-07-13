// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `CliBuilder` (T005/T029).
//!
//! `CliBuilder` collects `CliCommandRegistration` items from the global
//! `inventory` registry and constructs a `clap::Command` tree. Each
//! registration becomes a SubCommand of the top-level command; argument
//! metadata is mapped to clap args per the rules in `design.md`:
//! - `CliArgType::Path`  → positional
//! - `CliArgType::Body`  → `--name <VALUE>` option (with default_value if set)
//! - `CliArgType::State` → dropped (not surfaced on the CLI)

use crate::cli::{CliArgInfo, CliArgType, CliBuilder, CliCommandRegistration, GlobalArg};
use std::any::Any;
use std::sync::Arc;

// Test-only registration used by `test_builder_collects_commands` to
// verify the builder picks up `inventory::submit!` entries.
inventory::submit!(
    CliCommandRegistration::new(
        "builder_test_command",
        "v1",
        "Test command for builder_tests",
        "builder_test_handler"
    )
    .with_args(&[
        CliArgInfo::new("id", "Resource ID", CliArgType::Path, true, None),
        CliArgInfo::new("limit", "Max results", CliArgType::Body, false, Some("10"),),
        // State args must be dropped by the builder.
        CliArgInfo::new("state", "App state", CliArgType::State, true, None),
    ])
);

// ============================================================================
// T005: CliBuilder
// ============================================================================

/// `CliBuilder::new()` returns a fresh builder with no injected state.
/// `Default::default()` must be equivalent.
#[test]
fn test_builder_new_is_empty() {
    let _from_new = CliBuilder::new();
    let _from_default = CliBuilder::default();
    // Both construction paths must produce a buildable builder — exercise
    // build() to confirm no panics on the empty/default path.
    let cmd_from_new = CliBuilder::new().build();
    let cmd_from_default = CliBuilder::default().build();
    // Both must produce a `clap::Command` with the same name (top-level
    // program name is constant).
    assert_eq!(cmd_from_new.get_name(), cmd_from_default.get_name());
}

/// `CliBuilder::build()` must surface every registered
/// `CliCommandRegistration` as a SubCommand and translate argument
/// metadata into clap args per the design rules.
#[test]
fn test_builder_collects_commands() {
    let cmd = CliBuilder::new().build();

    // The statically-submitted test command must appear as a subcommand.
    let sub = cmd
        .find_subcommand("builder_test_command")
        .expect("builder_test_command must be a subcommand");
    // `get_about()` returns `Option<&StyledStr>`; flatten to a plain
    // string for the assertion.
    let about_str = sub.get_about().map(|s| s.to_string()).unwrap_or_default();
    assert_eq!(about_str, "Test command for builder_tests");

    // Path arg (`id`) becomes a positional. Count positionals and verify
    // the `id` positional is present.
    let positionals: Vec<_> = sub.get_positionals().collect();
    assert_eq!(
        positionals.len(),
        1,
        "expected exactly 1 positional (id); State must be dropped, Body is an option"
    );
    assert_eq!(positionals[0].get_id().as_str(), "id");

    // Body arg (`limit`) becomes an option with the configured default.
    let limit_arg = sub
        .get_arguments()
        .find(|a| a.get_id().as_str() == "limit")
        .expect("limit option must be present");
    // clap stores `--name` long flag for Body args.
    assert!(
        limit_arg.get_long().is_some(),
        "Body arg `limit` must have a --long flag"
    );
    // `get_default_values()` returns `&[clap::builder::OsStr]`; convert to
    // strings for the assertion.
    let defaults: Vec<String> = limit_arg
        .get_default_values()
        .iter()
        .map(|v| v.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        defaults,
        vec!["10".to_string()],
        "limit default must match CliArgInfo.default"
    );

    // State arg (`state`) must NOT appear as a clap arg.
    let state_present = sub.get_arguments().any(|a| a.get_id().as_str() == "state");
    assert!(!state_present, "State arg must not be surfaced on the CLI");
}

// ============================================================================
// T029: CliBuilder::with_dependencies — state injection
// ============================================================================

/// Test-only state type used to verify `with_dependencies` actually stores
/// the supplied state and makes it retrievable (via `state()`) for handler
/// invocation. Lives only in this test module.
struct TestAppState {
    counter: u32,
    label: &'static str,
}

/// Verify `with_dependencies(state)` stores the supplied state in the
/// builder so that handlers (T006+) can downcast it at call time. Also
/// verifies `build()` succeeds after state injection — the framework's
/// "constructor pattern 3" (full DI).
#[test]
fn test_builder_with_dependencies_injects_state() {
    // 1. `new()` starts with no state.
    let empty = CliBuilder::new();
    assert!(
        empty.state().is_none(),
        "CliBuilder::new() must start with no state"
    );

    // 2. `with_dependencies(state)` injects state.
    let state: Arc<dyn Any + Send + Sync> = Arc::new(TestAppState {
        counter: 42,
        label: "test",
    });
    let builder = CliBuilder::with_dependencies(state);
    let state_ref = builder
        .state()
        .expect("state must be present after with_dependencies");

    // 3. State is downcastable to the original concrete type — confirms
    //    the Arc<dyn Any + Send + Sync> round-trips correctly.
    let typed = state_ref
        .downcast_ref::<TestAppState>()
        .expect("downcast to TestAppState must succeed");
    assert_eq!(typed.counter, 42);
    assert_eq!(typed.label, "test");

    // 4. `build()` must succeed after state injection (no panic, no error).
    let _cmd = builder.build();
}

// ============================================================================
// T021: CliBuilder 自动注入 docs 子命令（docs feature）
// ============================================================================

/// 当 `docs` feature 启用时，`CliBuilder::build()` 必须自动 append
/// 名为 `docs` 的 SubCommand（来自 [`crate::cli::docs_subcommand`]
/// 模块），用户无需手动注册。
#[cfg(feature = "docs")]
#[test]
fn test_builder_includes_docs_subcommand() {
    let cmd = CliBuilder::new().build();
    let docs_sub = cmd
        .find_subcommand("docs")
        .expect("docs feature 启用时 build() 必须包含 docs 子命令");
    // 验证子命令名称与 about 描述
    let about_str = docs_sub
        .get_about()
        .map(|s| s.to_string())
        .unwrap_or_default();
    assert!(
        about_str.contains("documentation") || about_str.contains("文档"),
        "docs 子命令 about 必须包含文档描述，实际: {}",
        about_str
    );
    // 验证 --format 参数被注入
    assert!(
        docs_sub
            .get_arguments()
            .any(|a| a.get_id().as_str() == "format"),
        "注入的 docs 子命令必须含 --format 参数"
    );
    // 验证 --output 参数被注入
    assert!(
        docs_sub
            .get_arguments()
            .any(|a| a.get_id().as_str() == "output"),
        "注入的 docs 子命令必须含 --output 参数"
    );
}

/// 当 `docs` feature 未启用时（仅 `cli` feature），`CliBuilder::build()`
/// **不得**包含 `docs` 子命令——验证特性门控正确性，避免误注入。
#[cfg(not(feature = "docs"))]
#[test]
fn test_builder_excludes_docs_subcommand_when_docs_disabled() {
    let cmd = CliBuilder::new().build();
    assert!(
        cmd.find_subcommand("docs").is_none(),
        "docs feature 未启用时不应注入 docs 子命令"
    );
}

// ============================================================================
// GlobalArg + CliBuilder::with_global_arg
// ============================================================================

/// `GlobalArg::new` creates a global arg with sensible defaults: `global`
/// is `true`, `help` is empty, and `long`/`short`/`default_value` are `None`.
#[test]
fn test_global_arg_new_defaults() {
    let arg = GlobalArg::new("db");
    assert_eq!(arg.name, "db");
    assert!(arg.long.is_none());
    assert!(arg.short.is_none());
    assert!(arg.default_value.is_none());
    assert_eq!(arg.help, "");
    assert!(arg.global, "global should default to true");
}

/// Builder methods on `GlobalArg` set fields correctly.
#[test]
fn test_global_arg_builder_methods() {
    let arg = GlobalArg::new("config")
        .long("config")
        .short('c')
        .default_value("/etc/app.toml")
        .help("Path to config file")
        .global(false);
    assert_eq!(arg.name, "config");
    assert_eq!(arg.long, Some("config"));
    assert_eq!(arg.short, Some('c'));
    assert_eq!(arg.default_value.as_deref(), Some("/etc/app.toml"));
    assert_eq!(arg.help, "Path to config file");
    assert!(!arg.global, "global should be false after .global(false)");
}

/// `CliBuilder::with_global_arg` adds a global arg to the built command.
/// The arg must appear in the root command's arguments.
#[test]
fn test_builder_with_global_arg_appears_in_root() {
    let cmd = CliBuilder::new()
        .with_name("test_app")
        .with_global_arg(
            GlobalArg::new("db")
                .long("db")
                .default_value("./default.lbug")
                .help("Database path"),
        )
        .build();

    let db_arg = cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "db")
        .expect("global arg 'db' must appear in root command");
    assert_eq!(
        db_arg.get_long(),
        Some("db"),
        "long flag must be --db"
    );
    let defaults: Vec<String> = db_arg
        .get_default_values()
        .iter()
        .map(|v| v.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        defaults,
        vec!["./default.lbug".to_string()],
        "default value must match"
    );
}

/// `with_global_arg` can be chained to add multiple global args.
#[test]
fn test_builder_with_multiple_global_args() {
    let cmd = CliBuilder::new()
        .with_global_arg(GlobalArg::new("db").long("db").default_value("./db"))
        .with_global_arg(GlobalArg::new("verbose").short('v').help("Verbose output"))
        .build();

    assert!(
        cmd.get_arguments().any(|a| a.get_id().as_str() == "db"),
        "db arg must be present"
    );
    assert!(
        cmd.get_arguments().any(|a| a.get_id().as_str() == "verbose"),
        "verbose arg must be present"
    );
}

/// Global args with `global(true)` are inherited by subcommands — the
/// value can be provided at the subcommand level.
#[test]
fn test_global_arg_inherited_by_subcommand() {
    let cmd = CliBuilder::new()
        .with_name("test_app")
        .with_global_arg(GlobalArg::new("db").long("db").default_value("./default"))
        .build();

    // Parse args with --db provided at the top level (before subcommand).
    let matches = cmd.try_get_matches_from(["test_app", "--db", "/custom/path", "builder_test_command", "my_id"]);
    assert!(matches.is_ok(), "parsing should succeed: {:?}", matches.err());
    let matches = matches.unwrap();

    let sub = matches.subcommand_matches("builder_test_command").unwrap();
    let db_val = sub.get_one::<String>("db");
    assert_eq!(
        db_val.map(|s| s.as_str()),
        Some("/custom/path"),
        "global arg value must be accessible from subcommand matches"
    );
}

/// Global arg with no explicit `long` defaults to using `name` as the long flag.
#[test]
fn test_global_arg_long_defaults_to_name() {
    let arg = GlobalArg::new("config").help("Config path");
    let cmd = CliBuilder::new().with_global_arg(arg).build();
    let config_arg = cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "config")
        .expect("config arg must be present");
    assert_eq!(
        config_arg.get_long(),
        Some("config"),
        "long flag should default to name when not explicitly set"
    );
}

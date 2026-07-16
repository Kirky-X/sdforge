// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! CLI dispatch — routes a `clap::ArgMatches` to the matching forge handler.
//!
//! This module closes the dispatch loop opened by `CliBuilder::build()`:
//! build() materializes a `clap::Command` from inventory, the user-supplied
//! `main()` runs `get_matches()` to produce `ArgMatches`, and `dispatch()`
//! finds the matching `CliHandlerRegistration`, builds the `HandlerArgs`
//! from subcommand matches, and invokes the handler.
//!
//! Returns the handler's `serde_json::Value` output for the caller to
//! smart-extract via `crate::core::extract_value`.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::cli::{CliArgType, CliCommandRegistration, CliHandlerRegistration};
use crate::core::{ApiError, HandlerArgs, HandlerOutput, HandlerState};

/// Dispatch the selected subcommand to its registered forge handler.
///
/// Looks up the subcommand name in `CliHandlerRegistration`, builds the
/// `HandlerArgs` map from the subcommand matches (Path/Body args only;
/// State args are skipped and resolved by the handler via `downcast_state`),
/// and invokes the handler with the supplied `state`.
///
/// Returns `(command_name, Value)` on success — the caller decides how to
/// format the value (typically via `extract_value`).
///
/// # Errors
/// - `ApiError::Internal` when no subcommand was supplied (bare `prog`).
/// - `ApiError::NotFound` when the subcommand name has no registered handler.
/// - `ApiError::InvalidInput` when a required argument is missing.
/// - Whatever `ApiError` the handler itself returns.
pub async fn dispatch(
    matches: &clap::ArgMatches,
    state: HandlerState,
) -> Result<(String, HandlerOutput), ApiError> {
    let (name, sub) = matches.subcommand().ok_or_else(|| {
        ApiError::internal_error(
            "no subcommand supplied (run with --help to see available commands)",
            "cli.dispatch.no_subcommand",
        )
    })?;

    let handler_reg = find_handler(name)?;
    let cmd_reg = find_command(name)?;
    let args = build_args(sub, cmd_reg);
    let value = (handler_reg.handler)(args, state).await?;
    Ok((name.to_string(), value))
}

/// Look up a `CliHandlerRegistration` by command name.
fn find_handler(name: &str) -> Result<&'static CliHandlerRegistration, ApiError> {
    handler_lookup()
        .get(name)
        .copied()
        .ok_or_else(|| ApiError::NotFound {
            resource: "cli_command".to_string(),
            resource_id: Some(name.to_string()),
        })
}

/// Look up a `CliCommandRegistration` by command name (used to drive
/// `build_args` — which arguments to extract from matches).
fn find_command(name: &str) -> Result<&'static CliCommandRegistration, ApiError> {
    command_lookup()
        .get(name)
        .copied()
        .ok_or_else(|| ApiError::NotFound {
            resource: "cli_command".to_string(),
            resource_id: Some(name.to_string()),
        })
}

/// Build the `HandlerArgs` map from the subcommand matches.
///
/// Iterates `cmd_reg.args`:
/// - `Path` → `args[name] = sub.get_one::<String>(name)`
/// - `Body` → `args[name] = sub.get_one::<String>(name)` (clap renders `--name VALUE` the same way)
/// - `State` → skipped (resolved via `downcast_state` at handler call time)
fn build_args(sub: &clap::ArgMatches, cmd_reg: &CliCommandRegistration) -> HandlerArgs {
    let mut args: HandlerArgs = HashMap::new();
    for arg in cmd_reg.args {
        if matches!(arg.arg_type, CliArgType::State) {
            continue;
        }
        if let Some(v) = sub.get_one::<String>(arg.name) {
            args.insert(arg.name.to_string(), v.clone());
        }
    }
    args
}

/// Lazy-built lookup table: `name → CliHandlerRegistration`.
///
/// Built once from `inventory::iter` on first `dispatch` call, then reused
/// (OnceLock semantics — O(1) lookup after the first invocation).
fn handler_lookup() -> &'static HashMap<&'static str, &'static CliHandlerRegistration> {
    static HANDLERS: OnceLock<HashMap<&'static str, &'static CliHandlerRegistration>> =
        OnceLock::new();
    HANDLERS.get_or_init(|| {
        inventory::iter::<CliHandlerRegistration>()
            .map(|r| (r.name, r))
            .collect()
    })
}

/// Lazy-built lookup table: `name → CliCommandRegistration`.
fn command_lookup() -> &'static HashMap<&'static str, &'static CliCommandRegistration> {
    static COMMANDS: OnceLock<HashMap<&'static str, &'static CliCommandRegistration>> =
        OnceLock::new();
    COMMANDS.get_or_init(|| {
        inventory::iter::<CliCommandRegistration>()
            .map(|r| (r.name, r))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Command;
    use serde_json::json;

    /// Test handler used in the dispatch tests — emits a String return.
    fn greet_handler(args: HandlerArgs, _state: HandlerState) -> crate::core::HandlerFuture {
        let name = args
            .get("name")
            .cloned()
            .unwrap_or_else(|| "world".to_string());
        Box::pin(async move { Ok(json!(format!("Hello, {}!", name))) })
    }

    inventory::submit! {
        CliHandlerRegistration {
            name: "dispatch_test_greet",
            handler: greet_handler,
        }
    }

    inventory::submit! {
        CliCommandRegistration::new(
            "dispatch_test_greet",
            "1.0",
            "Test greet for dispatch",
            "greet_handler",
        ).with_args(&[
            crate::cli::CliArgInfo::new("name", "", crate::cli::CliArgType::Body, false, None),
        ])
    }

    fn build_test_command() -> Command {
        Command::new("test_prog").subcommand(
            Command::new("dispatch_test_greet")
                .arg(clap::Arg::new("name").long("name").required(false)),
        )
    }

    #[tokio::test]
    async fn dispatch_routes_to_registered_handler() {
        let cmd = build_test_command();
        let matches = cmd.get_matches_from(["test_prog", "dispatch_test_greet", "--name", "alice"]);
        let (name, value) = dispatch(&matches, None).await.unwrap();
        assert_eq!(name, "dispatch_test_greet");
        assert_eq!(value, json!("Hello, alice!"));
    }

    #[tokio::test]
    async fn dispatch_uses_default_when_arg_missing() {
        let cmd = build_test_command();
        let matches = cmd.get_matches_from(["test_prog", "dispatch_test_greet"]);
        let (_name, value) = dispatch(&matches, None).await.unwrap();
        assert_eq!(value, json!("Hello, world!"));
    }

    #[tokio::test]
    async fn dispatch_no_subcommand_returns_internal_error() {
        let cmd = Command::new("test_prog");
        let matches = cmd.get_matches_from(["test_prog"]);
        let err = dispatch(&matches, None).await.unwrap_err();
        assert!(matches!(err, ApiError::Internal { .. }));
    }

    #[tokio::test]
    async fn dispatch_unknown_command_returns_not_found() {
        let cmd = Command::new("test_prog").subcommand(Command::new("unknown_cmd"));
        let matches = cmd.get_matches_from(["test_prog", "unknown_cmd"]);
        let err = dispatch(&matches, None).await.unwrap_err();
        assert!(matches!(err, ApiError::NotFound { .. }));
    }

    #[test]
    fn handler_lookup_is_cached() {
        let a = handler_lookup();
        let b = handler_lookup();
        assert!(std::ptr::eq(a, b));
        assert!(a.contains_key("dispatch_test_greet"));
    }

    #[test]
    fn command_lookup_is_cached() {
        let a = command_lookup();
        let b = command_lookup();
        assert!(std::ptr::eq(a, b));
        assert!(a.contains_key("dispatch_test_greet"));
    }
}

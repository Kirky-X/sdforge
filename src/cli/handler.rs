// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `CliHandlerRegistration` — closure-based handler registration for CLI
//! commands.
//!
//! Pairing a `CliCommandRegistration` (metadata: name, args, version) with
//! an actual invocable handler is done via a separate inventory entry. The
//! macro emits both `inventory::submit!(CliCommandRegistration { ... })`
//! and `inventory::submit!(CliHandlerRegistration { ... })` for each
//! `#[forge(cli = true)]` function.
//!
//! The handler uses the unified `core::HandlerFn` signature
//! `(HandlerArgs, HandlerState) -> HandlerFuture`, returning
//! `Result<Value, ApiError>` — the same contract shared with gRPC, so a single
//! `#[forge]` function is callable from either protocol with zero duplication.

use crate::core::HandlerFn;

/// Static registration pairing a command name with its handler closure.
///
/// Looked up at call time by `name` (matching `CliCommandRegistration::name`)
/// to dispatch the user's selected subcommand. Submitted at compile time
/// via `inventory::submit!` from the `#[forge]` macro.
#[derive(Debug)]
pub struct CliHandlerRegistration {
    /// Command name this handler serves — must match the paired
    /// `CliCommandRegistration::name`.
    pub name: &'static str,
    /// Unified handler pointer (`core::HandlerFn`). The macro-generated fn
    /// extracts CLI args from `HandlerArgs`, awaits the forge function, and
    /// serializes its return value to `serde_json::Value`.
    pub handler: HandlerFn,
}

inventory::collect!(CliHandlerRegistration);

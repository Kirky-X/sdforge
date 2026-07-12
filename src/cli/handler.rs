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
//! The handler signature takes a `HashMap<String, String>` (the string-typed
//! argument values parsed by clap) and returns a pinned boxed future. This
//! mirrors the design trade-off noted in `design.md` (A3): type-safety is
//! traded for inventory simplicity — argument deserialization happens inside
//! the macro-generated closure.

use crate::prelude::ApiError;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Type alias for the handler function pointer.
///
/// `fn(HashMap<String, String>) -> Pin<Box<dyn Future<Output = Result<(), ApiError>> + Send + 'static>>`
pub type CliHandlerFn = fn(
    HashMap<String, String>,
) -> Pin<Box<dyn Future<Output = Result<(), ApiError>> + Send + 'static>>;

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
    /// Function pointer wrapping the macro-generated async closure.
    /// The closure receives the parsed CLI args as a `HashMap<String, String>`
    /// and resolves to `Result<(), ApiError>`.
    pub handler: CliHandlerFn,
}

inventory::collect!(CliHandlerRegistration);

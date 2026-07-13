// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! CLI protocol integration for SDForge.
//!
//! This module is gated by the `cli` feature and provides the same compile-time
//! `inventory` registration pattern used by HTTP/MCP/WebSocket/gRPC. The
//! `#[forge]` macro (when `cli = true`) emits
//! `inventory::submit!(CliCommandRegistration { ... })` plus a paired
//! `CliHandlerRegistration`. At runtime, \[`CliBuilder`\] collects the
//! registrations and constructs a `clap::Command`.

// ============================================================================
// T002: CliArgType + CliArgInfo
// ============================================================================

/// Classification of a CLI argument's source.
///
/// Mirrors the HTTP/MCP parameter kinds so the `#[forge]` macro can
/// reuse its existing `ParamInfo` infrastructure when emitting CLI
/// registrations. `State` arguments are never exposed to the end user on
/// the command line — they are injected by `CliBuilder::with_dependencies`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliArgType {
    /// Path-style positional argument (e.g. `<id>`).
    Path,
    /// Body-style option argument (e.g. `--name <VALUE>`).
    Body,
    /// Injected application state — not surfaced on the CLI itself.
    State,
}

/// Static metadata for a single CLI argument.
///
/// Constructed at compile time by the `#[forge]` macro and collected
/// into a `&'static [CliArgInfo]` on `CliCommandRegistration`. All fields
/// are `&'static str` / `Option<&'static str>` so the struct is `Copy` and
/// can live in read-only memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliArgInfo {
    /// Argument name as it appears on the command line.
    pub name: &'static str,
    /// Human-readable description used in `--help` output.
    pub description: &'static str,
    /// Source classification (Path/Body/State) — drives clap arg shape.
    pub arg_type: CliArgType,
    /// Whether the argument must be supplied by the user.
    pub required: bool,
    /// Default value rendered by clap when the argument is omitted.
    pub default: Option<&'static str>,
}

// ============================================================================
// T003: CliCommandRegistration
// ============================================================================

/// Static metadata for a CLI command, registered at compile time via
/// `inventory::submit!` and collected by `CliBuilder::build()`.
///
/// All fields are `&'static` so the registration lives in read-only memory
/// and the struct is `Copy`. The `args` slice is built by the
/// `#[forge]` macro from the function's parameter list (Path/Body
/// parameters become `CliArgInfo` entries; State parameters are dropped).
#[derive(Debug, Clone, Copy)]
pub struct CliCommandRegistration {
    /// Command name as the user types it (e.g. `echo`, `list`).
    pub name: &'static str,
    /// Semver-style version string surfaced in `--version`.
    pub version: &'static str,
    /// One-line description rendered in the parent command's `--help`.
    pub description: &'static str,
    /// Symbol name of the handler function — used to look up the paired
    /// `CliHandlerRegistration` at runtime.
    pub handler_fn_name: &'static str,
    /// Static slice of argument metadata, sorted in declaration order.
    pub args: &'static [CliArgInfo],
}

mod cli_impl;

// ============================================================================
// T004: inventory collect — compile-time registration of CLI commands.
// ============================================================================
//
// Mirrors the registration pattern in src/http/mod.rs and src/mcp/mod.rs.
// `inventory::collect!` declares the type as inventory-collectable; the
// `#[forge]` macro (T008) emits `inventory::submit!` blocks at
// call sites. At runtime, `CliBuilder::build()` (T005) iterates this
// registry to construct the `clap::Command` tree.
inventory::collect!(CliCommandRegistration);

// ============================================================================
// GlobalArg — declarative global CLI argument (avoids direct clap dep)
// ============================================================================

/// A global CLI argument that applies to all subcommands.
///
/// Downstream crates construct this via the builder pattern and pass it to
/// [`CliBuilder::with_global_arg`]. sdforge translates it into a
/// `clap::Arg` with `global(true)` at `build()` time, so callers never
/// need to depend on `clap` directly.
#[derive(Debug, Clone)]
pub struct GlobalArg {
    /// Argument identifier (used with `matches.get_one::<String>(name)`).
    pub name: &'static str,
    /// Long flag (e.g. `"db"` → `--db`). Defaults to `name` if `None`.
    pub long: Option<&'static str>,
    /// Short flag (e.g. `'d'` → `-d`). `None` skips the short flag.
    pub short: Option<char>,
    /// Default value rendered by clap when the argument is omitted.
    pub default_value: Option<String>,
    /// Help text shown in `--help` output.
    pub help: &'static str,
    /// Whether the argument is inherited by subcommands (clap `global`).
    pub global: bool,
}

impl GlobalArg {
    /// Create a new global arg with the given name and empty defaults.
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            long: None,
            short: None,
            default_value: None,
            help: "",
            global: true,
        }
    }

    /// Set the long flag (defaults to `name` when not set).
    #[must_use]
    pub fn long(mut self, long: &'static str) -> Self {
        self.long = Some(long);
        self
    }

    /// Set the short flag (e.g. `'d'` → `-d`).
    #[must_use]
    pub fn short(mut self, short: char) -> Self {
        self.short = Some(short);
        self
    }

    /// Set the default value when the argument is omitted.
    /// Accepts `impl Into<String>` so both `&'static str` and runtime
    /// `String` values work (e.g. `DEFAULT_DEBOUNCE_MS.to_string()`).
    #[must_use]
    pub fn default_value(mut self, default: impl Into<String>) -> Self {
        self.default_value = Some(default.into());
        self
    }

    /// Set the help text.
    #[must_use]
    pub fn help(mut self, help: &'static str) -> Self {
        self.help = help;
        self
    }

    /// Override the `global` flag (defaults to `true`).
    #[must_use]
    pub fn global(mut self, global: bool) -> Self {
        self.global = global;
        self
    }

    /// Translate to a `clap::Arg` at build time.
    pub(crate) fn to_clap_arg(&self) -> clap::Arg {
        let long = self.long.unwrap_or(self.name);
        let mut arg = clap::Arg::new(self.name)
            .long(long)
            .help(self.help)
            .global(self.global);
        if let Some(short) = self.short {
            arg = arg.short(short);
        }
        if let Some(default) = &self.default_value {
            arg = arg.default_value(default);
        }
        arg
    }
}

// ============================================================================
// T005: CliBuilder — runtime collector → clap::Command
// ============================================================================
pub mod builder;

pub use builder::CliBuilder;

// ============================================================================
// T020: docs subcommand — definition + handler (docs feature only)
// ============================================================================
#[cfg(feature = "docs")]
pub mod docs_subcommand;

#[cfg(feature = "docs")]
pub use docs_subcommand::{docs_subcommand, docs_subcommand_definition};

// ============================================================================
// T006: CliHandlerRegistration — closure-based handler dispatch
// ============================================================================
pub mod handler;

pub use handler::CliHandlerRegistration;

#[cfg(test)]
mod tests;

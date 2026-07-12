// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! CLI protocol integration for SDForge.
//!
//! This module is gated by the `cli` feature and provides the same compile-time
//! `inventory` registration pattern used by HTTP/MCP/WebSocket/gRPC. The
//! `#[service_api]` macro (when `cli = true`) emits
//! `inventory::submit!(CliCommandRegistration { ... })` plus a paired
//! `CliHandlerRegistration`. At runtime, \[`CliBuilder`\] collects the
//! registrations and constructs a `clap::Command`.

// ============================================================================
// T002: CliArgType + CliArgInfo
// ============================================================================

/// Classification of a CLI argument's source.
///
/// Mirrors the HTTP/MCP parameter kinds so the `#[service_api]` macro can
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
/// Constructed at compile time by the `#[service_api]` macro and collected
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
/// `#[service_api]` macro from the function's parameter list (Path/Body
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
// `#[service_api]` macro (T008) emits `inventory::submit!` blocks at
// call sites. At runtime, `CliBuilder::build()` (T005) iterates this
// registry to construct the `clap::Command` tree.
inventory::collect!(CliCommandRegistration);

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

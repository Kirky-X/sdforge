// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! CLI protocol integration for SDForge.
//!
//! This module is gated by the `cli` feature and provides the same compile-time
//! `inventory` registration pattern used by HTTP/MCP/WebSocket/gRPC. The
//! `#[service_api]` macro (when `cli = true`) emits
//! `inventory::submit!(CliCommandRegistration { ... })` plus a paired
//! `CliHandlerRegistration`. At runtime, [`CliBuilder`] collects the
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
#[derive(Debug, Clone, Copy)]
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

impl CliArgInfo {
    /// Construct a new `CliArgInfo`.
    ///
    /// Marked `const fn` so the `#[service_api]` macro can build argument
    /// arrays at compile time without runtime cost.
    pub const fn new(
        name: &'static str,
        description: &'static str,
        arg_type: CliArgType,
        required: bool,
        default: Option<&'static str>,
    ) -> Self {
        Self {
            name,
            description,
            arg_type,
            required,
            default,
        }
    }
}

#[cfg(test)]
mod tests;

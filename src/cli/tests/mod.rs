// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Test suites for the `cli` module.
//!
//! Organization mirrors the other protocol modules (http/mcp/grpc):
//! - `trait_tests`: registration primitives + inventory collection
//! - `builder_tests`: `CliBuilder` construction and state injection
//! - `handler_tests`: `CliHandlerRegistration` closure invocation
//! - `macro_integration_tests`: `#[forge(cli = true)]` end-to-end
//! - `integration_tests`: `init_all_plugins` CLI inventory linking

mod builder_tests;
#[cfg(feature = "docs")]
mod docs_subcommand_tests;
mod handler_tests;
mod integration_tests;
mod macro_integration_tests;
mod trait_tests;

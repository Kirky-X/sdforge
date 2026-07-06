// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Test suites for the `cli` module.
//!
//! Organization mirrors the other protocol modules (http/mcp/grpc):
//! - `trait_tests`: registration primitives + inventory collection
//! - `builder_tests`: `CliBuilder` construction and state injection
//! - `handler_tests`: `CliHandlerRegistration` closure invocation
//! - `macro_integration_tests`: `#[service_api(cli = true)]` end-to-end

mod builder_tests;
mod handler_tests;
mod macro_integration_tests;
mod trait_tests;

// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T007: `cli = true` is a legal `#[service_api]` argument.
//!
//! Verifies the macro accepts the new `cli` boolean parameter and emits a
//! compilable expansion. The expansion itself is gated by `#[cfg(feature =
//! "cli")]` so the trybuild test crate (which does not enable `cli`) still
//! type-checks.
use sdforge_macros::service_api;

#[service_api(name = "test_cli_arg_present", version = "v1", cli = true)]
async fn test_cli_arg_present() -> String {
    "hello".to_string()
}

#[service_api(name = "test_cli_arg_false", version = "v1", cli = false)]
async fn test_cli_arg_false() -> String {
    "hello".to_string()
}

fn main() {}

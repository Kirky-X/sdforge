// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T008: `#[forge(cli = true)]` with path + body parameters compiles.
//!
//! The macro emits `CliCommandRegistration` + `CliHandlerRegistration`
//! inventory submissions gated by `#[cfg(feature = "cli")]`. The trybuild
//! test crate does not enable `cli`, so the generated CLI items are
//! cfg-stripped and the test crate compiles cleanly. This indirectly
//! verifies the macro accepts the combination of arguments and emits
//! syntactically valid token streams.
use sdforge_macros::forge;

#[forge(
    name = "test_cli_gen",
    version = "v1",
    description = "Test CLI generation",
    path = "/users/:id",
    cli = true
)]
async fn test_cli_gen(id: u64, name: String) -> Result<String, String> {
    Ok(format!("User {} {}", id, name))
}

#[forge(name = "test_cli_no_args", version = "v1", cli = true)]
async fn test_cli_no_args() -> Result<String, String> {
    Ok("ok".to_string())
}

fn main() {}

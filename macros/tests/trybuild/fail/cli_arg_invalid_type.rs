// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T007: `cli` must be a boolean literal — passing a string is a compile error.
use sdforge_macros::forge;

#[forge(name = "test_cli_bad_type", version = "v1", cli = "not_bool")]
async fn test_cli_bad_type() -> String {
    "hello".to_string()
}

fn main() {}

// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use sdforge_macros::forge;

#[forge(name = "test")]
async fn test_no_version() -> String {
    "hello".to_string()
}

fn main() {}

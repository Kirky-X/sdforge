// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use sdforge_macros::forge;

#[forge(name = "test@invalid", version = "v1")]
async fn test_invalid_name() -> String {
    "hello".to_string()
}

fn main() {}

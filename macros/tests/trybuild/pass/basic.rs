// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use sdforge_macros::forge;

#[forge(name = "test_basic", version = "v1")]
async fn test_basic() -> String {
    "hello".to_string()
}

fn main() {}

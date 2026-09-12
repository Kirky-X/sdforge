// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// empty role values are rejected at the literal token.
use sdforge_macros::forge;

#[forge(name = "auth_empty", version = "v1", auth(role = ""))]
async fn demo() -> String {
    "hello".to_string()
}

fn main() {}

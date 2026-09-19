// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// unknown attribute diagnostics must point at the offending key token.
use sdforge_macros::forge;

#[forge(name = "typo_demo", version = "v1", nmae2 = "oops")]
async fn demo() -> String {
    "hello".to_string()
}

fn main() {}

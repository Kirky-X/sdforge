// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use sdforge_macros::forge;

#[forge(name = "test_tool", version = "v1", tool_name = "my_tool")]
async fn my_tool() -> String {
    "tool result".to_string()
}

fn main() {}

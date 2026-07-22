// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! M-1/LOW-1: `status` 必须在 100..=999 范围内 — 99 编译失败。
use sdforge_macros::forge;

#[forge(name = "test_status_out_of_range", version = "v1", status = 99)]
async fn test_status_out_of_range() -> String {
    "hello".to_string()
}

fn main() {}

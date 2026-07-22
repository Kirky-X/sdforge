// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T005: `status` 必须是 u16 字面量 — 传字符串编译失败。
use sdforge_macros::forge;

#[forge(name = "test_status_bad_type", version = "v1", status = "abc")]
async fn test_status_bad_type() -> String {
    "hello".to_string()
}

fn main() {}

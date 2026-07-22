// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T005: `status` 参数解析为 `Option<u16>`，合法 u16 字面量编译通过。
//! 仅测宏解析（不带 path/method，不触发 HTTP 生成，避免 sdforge 运行时依赖）。
use sdforge_macros::forge;

#[forge(name = "test_status_arg", version = "v1", status = 201)]
async fn test_status_arg() -> String {
    "created".to_string()
}

fn main() {}

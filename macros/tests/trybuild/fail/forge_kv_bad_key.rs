// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// 不能作键的 token（字符串字面量）在 kv 解析器报错且 Span 指向该 token
// （syn 迁移后的 Span 精确化契约；旧实现一律 call_site）。
use sdforge_macros::forge;

#[forge("bad" = "x")]
async fn demo() -> String {
    "hello".to_string()
}

fn main() {}

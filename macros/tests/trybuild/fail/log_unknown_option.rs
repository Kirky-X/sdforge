// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// #[forge::log] 未知名参数在宏展开期报错（Span 指向 offending 项）。
use sdforge_macros as forge;

#[forge::log(wat)]
fn demo() -> u32 {
    1
}

fn main() {
    let _ = demo();
}
